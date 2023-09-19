use core::hint::unreachable_unchecked;
use core::ops::{Add, DerefMut};

use embassy_nrf::{peripherals, spim};
use embassy_nrf::gpio::{AnyPin, Output};
use embassy_nrf::spim::Spim;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use nrf_softdevice::ble::Connection;
use rclite::Arc;

use crate::{ble_debug, SPI_EXPANDER_EVENTS};
use crate::common::ble::{SERVER, SPI_EXPANDER_LOCK_OWNER};
use crate::common::ble::services::SpiExpanderServiceEvent;
use crate::common::device::config::{BLE_SPI_EXPANDER_BUF, NUM_CONNECTIONS};
use crate::common::device::device_manager::{ExpanderPins, Irqs};
use crate::common::device::error::SpiExpanderError;
use crate::common::util::ble_debugger::{ble_debug_push, ConnectionDebug};

static TIMEOUT_CHANNEL: Channel<ThreadModeRawMutex, Connection, NUM_CONNECTIONS> =
    Channel::new();


static STATE: Mutex<ThreadModeRawMutex, Option<SpiExpanderState>> = Mutex::new(None);

struct SpiExpanderState {
    power: bool,
    size: usize,
    cs: u8,
    command: Command,
    mosi: [u8; BLE_SPI_EXPANDER_BUF],
}

impl SpiExpanderState {
    const fn new() -> Self {
        Self {
            power: false,
            size: 0,
            cs: 0,
            command: Command::Write,
            mosi: [0u8; BLE_SPI_EXPANDER_BUF],
        }
    }

    async fn exec(
        &self,
        pins: &Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
    ) -> Result<Option<[u8; BLE_SPI_EXPANDER_BUF]>, SpiExpanderError> {
        handle_write(&pins, self.command, &self.mosi[..self.size]).await
    }
}


#[derive(defmt::Format, Debug, Copy, Clone)]
enum Command {
    Write,
    Read,
    Transfer,
}

impl TryFrom<u8> for Command {
    type Error = SpiExpanderError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Command::Write),
            0x01 => Ok(Command::Read),
            0x02 => Ok(Command::Transfer),
            _ => Err(SpiExpanderError::InvalidCommand(value))
        }
    }
}

trait Expander {
    fn select(&mut self, num: u8);
}

impl Expander for [&mut Output<'_, AnyPin>; 3] {
    fn select(&mut self, num: u8) {
        let flags = [
            num & (1 << 0) != 0,
            num & (1 << 1) != 0,
            num & (1 << 2) != 0,
        ];

        self.iter_mut().zip(flags).for_each(|(pin, flag)| {
            if flag {
                pin.set_high();
            } else {
                pin.set_low();
            }
        });
    }
}

async fn authenticate(connection: &Connection) -> Result<(), SpiExpanderError> {
    let owner = SPI_EXPANDER_LOCK_OWNER.lock().await;
    if let Some((_, owning_connection)) = owner.as_ref() {
        if owning_connection == connection {
            Ok(())
        } else {
            Err(SpiExpanderError::MutexAcquiredByOtherClient)
        }
    } else {
        Err(SpiExpanderError::MutexNotLocked)
    }
}


async fn handle_set_cs(
    pins: &Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
    cs: u8,
) {
    let mut pins = pins.lock().await;
    let pins = pins.deref_mut();

    let mut cs_pins = [
        &mut pins.a0,
        &mut pins.a1,
        &mut pins.a2,
    ];
    cs_pins.select(cs);
}

async fn handle_write(
    pins: &Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
    command: Command,
    write_buf: &[u8],
) -> Result<Option<[u8; BLE_SPI_EXPANDER_BUF]>, SpiExpanderError> {
    let mut pins = pins.lock().await;
    let pins = pins.deref_mut();

    let mut spim_config = spim::Config::default();
    spim_config.frequency = pins.spim_config.frequency;
    spim_config.mode = pins.spim_config.mode;
    spim_config.orc = pins.spim_config.orc;

    let mut spi =
        Spim::new(&mut pins.spi_peripheral, Irqs, &mut pins.sck, &mut pins.miso, &mut pins.mosi, spim_config);

    let mut read_buf = [0u8; BLE_SPI_EXPANDER_BUF];

    match command {
        Command::Write => {
            spi.write(write_buf).await?;
            Ok(None)
        }
        Command::Read => {
            spi.read(&mut read_buf[..write_buf.len()]).await?;
            Ok(Some(read_buf))
        }
        Command::Transfer => {
            spi.transfer(&mut read_buf[..write_buf.len()], write_buf).await?;
            Ok(Some(read_buf))
        }
    }
}

async fn handle_mutex_acquire_release(
    next_lock_value: u8,
    connection: &Connection,
) -> Result<bool, SpiExpanderError> {
    let mut owner = SPI_EXPANDER_LOCK_OWNER.lock().await;

    match owner.as_mut() {
        None => {
            if next_lock_value == 0 {
                Err(SpiExpanderError::MutexReleaseNotLocked)
            } else {
                *owner = Some((Instant::now(), connection.clone()));
                Ok(true)
            }
        }
        Some((_, owning_connection)) if owning_connection == connection => {
            if next_lock_value == 0 {
                *owner = None;
                Ok(false)
            } else {
                Err(SpiExpanderError::MutexAcquireTwiceSameClient)
            }
        }
        _ => {
            Err(SpiExpanderError::MutexAcquiredByOtherClient)
        }
    }
}


async fn handle_power(
    pins: &Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
    on: bool,
) {
    let mut pins = pins.lock().await;
    let pins = pins.deref_mut();
    if on {
        pins.power_switch.set_high();
    } else {
        pins.power_switch.set_low();
    }
}

#[embassy_executor::task]
pub(crate) async fn expander_task(
    pins: Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
) {
    loop {
        let (connection, event) = SPI_EXPANDER_EVENTS.recv().await;

        match &event {
            SpiExpanderServiceEvent::LockWrite(value) => {
                match handle_mutex_acquire_release(*value, &connection).await {
                    Ok(locked) => {
                        if locked {
                            STATE.lock().await.replace(SpiExpanderState::new());
                            TIMEOUT_CHANNEL.send(connection.clone()).await;
                        }
                        connection.debug(format_args!("SPI Expander mutex locked: {}", locked));
                    }
                    Err(err) => {
                        connection.debug(format_args!("SPI Expander mutex error: {:?}", err));
                    }
                }
                continue;
            }
            _ => {
                if let Err(err) = authenticate(&connection).await {
                    connection.debug(format_args!("SPI Expander auth error: {:?}", err));
                    continue;
                }
            }
        }

        if STATE.lock().await.is_none() {
            connection.debug(format_args!("SPI Expander state is not initialized"));
            continue;
        }

        match event {
            SpiExpanderServiceEvent::LockWrite(_) => unsafe { unreachable_unchecked() },
            SpiExpanderServiceEvent::MosiWrite(buf) => {
                if let Some(state) = STATE.lock().await.as_mut() {
                    state.mosi = buf;
                    connection.debug(format_args!("SPI Expander mosi set"));
                }
            }
            SpiExpanderServiceEvent::CsWrite(cs) => {
                if cs > 7 {
                    let _ = connection.debug(format_args!("SPI Expander got an invalid CS: {}", cs));
                    continue;
                }
                if let Some(state) = STATE.lock().await.as_mut() {
                    state.cs = cs;
                    connection.debug(format_args!("CS set to {}", cs));
                    handle_set_cs(&pins, cs).await;
                }
            }
            SpiExpanderServiceEvent::CommandWrite(command) => {
                if let Some(state) = STATE.lock().await.as_mut() {
                    state.command = if let Ok(command) = Command::try_from(command) {
                        command
                    } else {
                        connection.debug(format_args!("Invalid SPI Expander command: {}", command));
                        continue;
                    };
                }

                if let Some(state) = STATE.lock().await.as_ref() {
                    match state.exec(&pins).await {
                        Ok(Some(read_buf)) => {
                            let _ = SERVER.get().spi_expander.miso_set(&read_buf);
                            ble_debug!("SPI Expander OK: stored read buffer");
                        }
                        Ok(None) => {
                            ble_debug!("SPI Expander OK: no read buffer");
                        }
                        Err(err) => {
                            let _ = ble_debug_push(Some(connection), format_args!("SPI Expander write error: {:?}", err));
                        }
                    }
                }
            }
            SpiExpanderServiceEvent::PowerWrite(on) => {
                if let Some(state) = STATE.lock().await.as_mut() {
                    state.power = on == 1;
                    handle_power(&pins, state.power).await;
                    connection.debug(format_args!("SPI Expander power set to {}", on));
                }
            }
            SpiExpanderServiceEvent::SizeWrite(size) => {
                if size > BLE_SPI_EXPANDER_BUF as u16 {
                    connection.debug(format_args!("SPI Expander invalid size: {}; max size: {}", size, BLE_SPI_EXPANDER_BUF));
                    continue;
                }
                if let Some(state) = STATE.lock().await.as_mut() {
                    state.size = size as usize;
                }
            }
        }
    }
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
pub(crate) async fn spi_expander_mutex_timeout_task(
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
            continue
        };

        if elapsed > Duration::from_secs(10) {
            handle_disconnect(&tracking_connection, pins.clone()).await;
            tracking_connection.debug(format_args!("SPI Expander mutex timeout"));
        } else {
            Timer::after(Duration::from_millis(100)).await;
            if TIMEOUT_CHANNEL.try_send(tracking_connection.clone()).is_err() {
                tracking_connection.debug(format_args!("Failed to reschedule connection"));
            }
        }
    }
}
