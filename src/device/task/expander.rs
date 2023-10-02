use core::ops::DerefMut;

use defmt::info;
use embassy_nrf::peripherals;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use nrf_softdevice::ble::Connection;
use rclite::Arc;

use crate::common::ble::{SERVER, SPI_EXPANDER_EVENTS};
use crate::common::ble::services::ExpanderServiceEvent;
use crate::common::device::pin_manager::ExpanderPins;
use crate::common::device::error::ExpanderError;
use crate::common::device::expander::{authenticate, EXPANDER_STATE, handle_expander_disconnect, handle_mutex_acquire_release, handle_power, handle_set_cs, TIMEOUT_TRACKER};
use crate::common::device::expander::expander_state::{ExpanderFlags, ExpanderState, ExpanderType};
use crate::common::util::ble_debugger::ConnectionDebug;

impl ExpanderState {}

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
        let (connection, event) = SPI_EXPANDER_EVENTS.receive().await;

        let mut expander_state = EXPANDER_STATE.lock().await;
        let expander_state = expander_state.deref_mut();

        if let ExpanderServiceEvent::ResultCccdWrite { .. } = event {
            continue;
        }

        let success_code = event.success_code();
        match handle_event(connection.clone(), event, expander_state, &pins).await {
            Ok(()) => {
                notify_result(&connection, success_code);
            }
            Err(ExpanderError::MutexNotLocked) | Err(ExpanderError::MutexAcquireTwiceSameClient) | Err(ExpanderError::MutexAcquiredByOtherClient) | Err(ExpanderError::MutexReleaseNotLocked) => {
                notify_result(&connection, i8::MIN + success_code);
                info!("Expander auth error");
            }
            Err(err) => {
                notify_result(&connection, -success_code);
                info!("Expander error: {:?}", err)
            }
        }
    }
}

async fn handle_event(
    connection: Connection,
    event: ExpanderServiceEvent,
    expander_state: &mut Option<ExpanderState>,
    pins: &Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
) -> Result<(), ExpanderError> {
    info!("Expander event: {:?}", event.success_code());
    match event {
        ExpanderServiceEvent::ResultCccdWrite { .. } => Ok(()),
        ExpanderServiceEvent::LockWrite(value) => match handle_mutex_acquire_release(value, &connection).await {
            // when locking anew, replace the old state
            Ok(expander_lock_type) => {
                let _ = expander_state.replace(ExpanderState::default_with_type(expander_lock_type));
                match expander_lock_type {
                    ExpanderType::NotSet => {
                        expander_state.take();
                        TIMEOUT_TRACKER.stop_tracking(&connection).await;
                        handle_power(pins, false).await;
                        handle_set_cs(pins, 0).await;
                        // handle_expander_disconnect()
                    }
                    ExpanderType::Spi | ExpanderType::I2c => {
                        TIMEOUT_TRACKER.register(connection.clone()).await
                    }
                }
                Ok(())
            }
            Err(err) => Err(err)
        },
        ExpanderServiceEvent::CsWrite(cs) => {
            authenticate(&connection).await?;
            if cs > 7 {
                return Err(ExpanderError::InvalidCs(cs));
            }
            let expander_state = expander_state.as_mut().ok_or(ExpanderError::StateNotInitialized)?;
            expander_state.flags.cs = Some(cs);
            handle_set_cs(pins, cs).await;
            info!("Expander CS set to {}", cs);
            Ok(())
        }

        ExpanderServiceEvent::PowerWrite(on) => {
            authenticate(&connection).await?;
            let expander_state = expander_state.as_mut().ok_or(ExpanderError::StateNotInitialized)?;
            expander_state.flags.power = Some(on == 1);

            handle_power(&pins, expander_state.flags.power.unwrap()).await;
            info!("Expander power set to {}", on);
            Ok(())
        }
        ExpanderServiceEvent::DataBundleWrite(data) => {
            match authenticate(&connection).await {
                Err(ExpanderError::MutexNotLocked) | Ok(()) => {
                    let expander_server = &SERVER.get().expander;

                    let flags = ExpanderFlags::try_from(&data[..])?;

                    let expander_lock_type = if let Some(lock_type) = flags.expander_lock_type {
                        lock_type
                    } else {
                        ExpanderType::try_from(expander_server.lock_get()?)?
                    };
                    // info!("Expander lock type: {:?}", expander_lock_type);

                    match handle_mutex_acquire_release(expander_lock_type as u8, &connection).await {
                        // forgive double-locking
                        Err(ExpanderError::MutexAcquireTwiceSameClient) => Ok(()),
                        Ok(_) => Ok(()),
                        Err(err) => Err(err)
                    }?;

                    TIMEOUT_TRACKER.register(connection.clone()).await;

                    expander_server.lock_set(&(expander_lock_type as u8))?;

                    if let Some(expander_state) = expander_state {
                        expander_state.update(data)?;
                        expander_state.flags.expander_lock_type = Some(expander_lock_type);
                    } else {
                        let mut state = ExpanderState::try_from(data)?;
                        state.flags.expander_lock_type = Some(expander_lock_type);
                        expander_state.replace(state);
                    }

                    let expander_state = expander_state.as_mut().ok_or(ExpanderError::StateNotInitialized)?;

                    if let Some(power) = expander_state.flags.power {
                        handle_power(pins, power).await;
                        expander_server.power_set(&1)?;
                    }
                    Timer::after(expander_state.flags.power_wait_duration).await;

                    if let Some(cs) = expander_state.flags.cs {
                        handle_set_cs(pins, cs).await;
                        expander_server.cs_set(&cs)?;
                    }
                    Timer::after(expander_state.flags.cs_wait_duration).await;

                    // info!("Expander flags: {:?}", expander_state.flags);

                    if let Some(response_buf) = expander_state.exec(pins).await? {
                        expander_server.miso_set(&response_buf)?;
                    }

                    Ok(())
                }
                err => err,
            }
        }
    }
}


#[embassy_executor::task]
pub(crate) async fn expander_mutex_timeout_task(
    pins: Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
) {
    loop {
        let _token = TIMEOUT_TRACKER.wait().await;
        for connection in Connection::iter() {
            if TIMEOUT_TRACKER.verify_timeout(&connection).await {
                handle_expander_disconnect(&connection, &pins).await;
                connection.debug(format_args!("Expander mutex timed out"));
            }
        }

        Timer::after(Duration::from_millis(1000)).await;
    }
}

