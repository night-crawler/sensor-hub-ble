use core::hint::unreachable_unchecked;

use defmt::info;
use embassy_nrf::peripherals;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use nrf_softdevice::ble::Connection;
use rclite::Arc;

use crate::common::ble::{SERVER, SPI_EXPANDER_EVENTS, SPI_EXPANDER_LOCK_OWNER};
use crate::common::ble::services::ExpanderServiceEvent;
use crate::common::device::config::{BLE_EXPANDER_BUF_SIZE, BLE_EXPANDER_TIMEOUT, NUM_CONNECTIONS};
use crate::common::device::device_manager::ExpanderPins;
use crate::common::device::error::ExpanderError;
use crate::common::device::expander::{authenticate, handle_i2c_write, handle_mutex_acquire_release, handle_power, handle_set_cs, handle_spi_write};
use crate::common::device::expander::command::Command;
use crate::common::device::expander::expander_state::{ExpanderState, ExpanderType};
use crate::common::util::ble_debugger::ConnectionDebug;

static TIMEOUT_CHANNEL: Channel<ThreadModeRawMutex, Connection, NUM_CONNECTIONS> =
    Channel::new();


static STATE: Mutex<ThreadModeRawMutex, Option<ExpanderState>> = Mutex::new(None);


impl ExpanderState {
    pub(crate) async fn exec(
        &self,
        pins: &Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
    ) -> Result<Option<[u8; BLE_EXPANDER_BUF_SIZE]>, ExpanderError> {
        match self.expander_type {
            ExpanderType::None => unsafe { unreachable_unchecked() },
            ExpanderType::Spi => {
                handle_spi_write(&pins, self.command, &self.mosi[..self.size]).await
            }
            ExpanderType::I2c => {
                handle_i2c_write(&pins, self.i2c_address, self.command, &self.mosi[..self.size]).await
            }
        }
    }
}

fn notify_result(connection: &Connection, result: i8) {
    if let Err(err) = SERVER.get().expander.result_notify(connection, &result) {
        connection.debug(format_args!("Failed to notify result: {:?}", err));
    }
}

#[embassy_executor::task]
pub(crate) async fn expander_task(
    pins: Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
) {
    loop {
        let (connection, event) = SPI_EXPANDER_EVENTS.recv().await;

        match &event {
            ExpanderServiceEvent::LockWrite(value) => {
                match handle_mutex_acquire_release(*value, &connection).await {
                    Ok(expander_type) => {
                        match expander_type {
                            ExpanderType::Spi | ExpanderType::I2c => {
                                STATE.lock().await.replace(ExpanderState::new_with_type(expander_type));
                                TIMEOUT_CHANNEL.send(connection.clone()).await;
                            }
                            _ => {
                                STATE.lock().await.take();
                            }
                        }
                        info!("Expander mutex locked: {:?}", expander_type);
                        notify_result(&connection, event.success_code());
                    }
                    Err(err) => {
                        info!("Expander mutex error: {:?}", err);
                        notify_result(&connection, event.error_code());
                    }
                }
                continue;
            }
            ExpanderServiceEvent::ResultCccdWrite { .. } => {
                continue;
            }
            _ => {
                if let Err(err) = authenticate(&connection).await {
                    connection.debug(format_args!("Expander auth error: {:?}", err));
                    notify_result(&connection, i8::MIN + event.success_code());
                    continue;
                }
            }
        }

        if STATE.lock().await.is_none() {
            connection.debug(format_args!("Expander state is not initialized"));
            continue;
        }

        let notify_code = process_event(event, &connection, &pins).await;
        notify_result(&connection, notify_code);
    }
}

async fn process_event(
    event: ExpanderServiceEvent,
    connection: &Connection,
    pins: &Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
) -> i8 {
    let mut result_code = event.error_code();

    match event {
        ExpanderServiceEvent::LockWrite(_) => unsafe { unreachable_unchecked() },
        ExpanderServiceEvent::MosiWrite(buf) => {
            if let Some(state) = STATE.lock().await.as_mut() {
                state.mosi = buf;
                info!("Expander MOSI set");
                result_code = event.success_code();
            }
        }
        ExpanderServiceEvent::CsWrite(cs) => {
            if cs > 7 {
                info!("Expander got an invalid CS: {}", cs);
                return result_code;
            }
            if let Some(state) = STATE.lock().await.as_mut() {
                state.cs = cs;
                handle_set_cs(&pins, cs).await;
                info!("CS set to {}", cs);
                result_code = event.success_code();
            }
        }
        ExpanderServiceEvent::CommandWrite(command) => {
            if let Some(state) = STATE.lock().await.as_mut() {
                state.command = if let Ok(command) = Command::try_from(command) {
                    command
                } else {
                    info!("Invalid Expander command: {}", command);
                    return result_code;
                };
            }

            if let Some(state) = STATE.lock().await.as_ref() {
                match state.exec(&pins).await {
                    Ok(Some(read_buf)) => {
                        let _ = SERVER.get().expander.miso_set(&read_buf);
                        info!("Expander OK: read buffer");
                        result_code = event.success_code();
                    }
                    Ok(None) => {
                        info!("Expander OK: no read buffer");
                        result_code = event.success_code();
                    }
                    Err(err) => {
                        info!("Expander exec error: {:?}", err);
                        return result_code;
                    }
                }
            }
        }
        ExpanderServiceEvent::PowerWrite(on) => {
            if let Some(state) = STATE.lock().await.as_mut() {
                state.power = on == 1;
                handle_power(&pins, state.power).await;
                info!("Expander power set to {}", on);
                result_code = event.success_code();
            }
        }
        ExpanderServiceEvent::SizeWrite(size) => {
            if size > BLE_EXPANDER_BUF_SIZE as u16 {
                info!("Expander invalid size: {}; max size: {}", size, BLE_EXPANDER_BUF_SIZE);
                return result_code;
            }
            if let Some(state) = STATE.lock().await.as_mut() {
                state.size = size as usize;
                info!("Expander size set to {}", size);
                result_code = event.success_code();
            }
        }
        ExpanderServiceEvent::AddressWrite(address) => {
            if let Some(state) = STATE.lock().await.as_mut() {
                state.i2c_address = address;
                info!("Expander I2C address set to {:#04x}", address);
                result_code = event.success_code();
            }
        }
        ExpanderServiceEvent::ResultCccdWrite { .. } => {
            result_code = event.success_code();
        }
    }

    result_code
}

pub(crate) async fn handle_disconnect(
    connection: &Connection,
    pins: Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
) {
    let mut owner = SPI_EXPANDER_LOCK_OWNER.lock().await;
    if let Some((_, owning_connection)) = owner.as_ref() {
        if owning_connection == connection {
            owner.take();
            STATE.lock().await.as_mut().take();
            handle_power(&pins, false).await;
            handle_set_cs(&pins, 0).await;
        }
    }
}


#[embassy_executor::task]
pub(crate) async fn expander_mutex_timeout_task(
    pins: Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
) {
    loop {
        let tracking_connection = TIMEOUT_CHANNEL.recv().await;

        let elapsed = if let Some((locked_at, owning_connection)) = SPI_EXPANDER_LOCK_OWNER.lock().await.as_ref() {
            if owning_connection != &tracking_connection {
                continue;
            }
            locked_at.elapsed()
        } else {
            continue;
        };

        if elapsed > BLE_EXPANDER_TIMEOUT {
            handle_disconnect(&tracking_connection, pins.clone()).await;
            tracking_connection.debug(format_args!("Expander mutex timeout"));
        } else {
            Timer::after(Duration::from_millis(100)).await;
            if TIMEOUT_CHANNEL.try_send(tracking_connection.clone()).is_err() {
                tracking_connection.debug(format_args!("Failed to reschedule connection"));
            }
        }
    }
}
