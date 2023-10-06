use core::ops::DerefMut;

use embassy_nrf::{peripherals, spim, twim};
use embassy_nrf::spim::Spim;
use embassy_nrf::twim::Twim;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use nrf_softdevice::ble::Connection;
use rclite::Arc;

use crate::common::ble::SPI_EXPANDER_LOCK_OWNER;
use crate::common::device::config::{BLE_EXPANDER_BUF_SIZE, BLE_EXPANDER_LOCK_TIMEOUT};
use crate::common::device::peripherals_manager::{ExpanderPins, Irqs};
use crate::common::device::error::ExpanderError;
use crate::common::device::expander::command::Command;
use crate::common::device::expander::expander_state::{ExpanderState, ExpanderType};
use crate::common::device::expander::ext::Expander;
use crate::common::util::timeout_tracker::TimeoutTracker;

pub(crate) mod expander_state;
pub(crate) mod command;
pub(crate) mod ext;


pub(crate) static EXPANDER_STATE: Mutex<ThreadModeRawMutex, Option<ExpanderState>> = Mutex::new(None);

pub(crate) static TIMEOUT_TRACKER: TimeoutTracker<Connection> = TimeoutTracker::new(BLE_EXPANDER_LOCK_TIMEOUT);


pub(crate) async fn authenticate(connection: &Connection) -> Result<(), ExpanderError> {
    let owner = SPI_EXPANDER_LOCK_OWNER.lock().await;
    if let Some(owning_connection) = owner.as_ref() {
        if owning_connection == connection {
            Ok(())
        } else {
            Err(ExpanderError::MutexAcquiredByOtherClient)
        }
    } else {
        Err(ExpanderError::MutexNotLocked)
    }
}

pub(crate) async fn handle_set_cs(
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

pub(crate) async fn handle_mutex_acquire_release(
    next_lock_value: u8,
    connection: &Connection,
) -> Result<ExpanderType, ExpanderError> {
    let mut owner = SPI_EXPANDER_LOCK_OWNER.lock().await;

    match owner.as_mut() {
        None => {
            if next_lock_value == 0 {
                Err(ExpanderError::MutexReleaseNotLocked)
            } else {
                *owner = Some(connection.clone());

                if next_lock_value == 1 {
                    Ok(ExpanderType::Spi)
                } else {
                    Ok(ExpanderType::I2c)
                }
            }
        }
        Some(owning_connection) if owning_connection == connection => {
            if next_lock_value == 0 {
                owner.take();
                Ok(ExpanderType::NotSet)
            } else {
                Err(ExpanderError::MutexAcquireTwiceSameClient)
            }
        }
        _ => {
            Err(ExpanderError::MutexAcquiredByOtherClient)
        }
    }
}

pub(crate) async fn handle_power(
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

pub(crate) async fn handle_spi_exec(
    pins: &Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
    command: Command,
    write_buf: &[u8],
) -> Result<Option<[u8; BLE_EXPANDER_BUF_SIZE]>, ExpanderError> {
    let mut pins = pins.lock().await;
    let pins = pins.deref_mut();

    let mut spim_config = spim::Config::default();
    spim_config.frequency = pins.spim_config.frequency;
    spim_config.mode = pins.spim_config.mode;
    spim_config.orc = pins.spim_config.orc;

    let mut spi =
        Spim::new(&mut pins.spi_peripheral, Irqs, &mut pins.sck, &mut pins.miso, &mut pins.mosi, spim_config);

    let mut read_buf = [0u8; BLE_EXPANDER_BUF_SIZE];

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


pub(crate) async fn handle_i2c_exec(
    pins: &Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
    address: u8,
    command: Command,
    write_buf: &[u8],
    size_read: usize
) -> Result<Option<[u8; BLE_EXPANDER_BUF_SIZE]>, ExpanderError> {
    let mut pins = pins.lock().await;
    let pins = pins.deref_mut();

    let mut i2c_config = twim::Config::default();
    i2c_config.frequency = pins.i2c_config.frequency;
    i2c_config.sda_high_drive = pins.i2c_config.sda_high_drive;
    i2c_config.sda_pullup = pins.i2c_config.sda_pullup;
    i2c_config.scl_high_drive = pins.i2c_config.scl_high_drive;
    i2c_config.scl_pullup = pins.i2c_config.scl_pullup;

    let mut i2c = Twim::new(&mut pins.i2c_peripheral, Irqs, &mut pins.sda, &mut pins.scl, i2c_config);

    let mut read_buf = [0u8; BLE_EXPANDER_BUF_SIZE];

    match command {
        Command::Write => {
            i2c.write(address, write_buf).await?;
            Ok(None)
        }
        Command::Read => {
            i2c.read(address, &mut read_buf[..size_read]).await?;
            Ok(Some(read_buf))
        }
        Command::Transfer => {
            i2c.write_read(address, write_buf, &mut read_buf[..size_read]).await?;
            Ok(Some(read_buf))
        }
    }
}


pub(crate) async fn handle_expander_disconnect(
    connection: &Connection,
    pins: &Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
) {
    let mut owner = SPI_EXPANDER_LOCK_OWNER.lock().await;
    if let Some(owning_connection) = owner.as_ref() {
        if owning_connection == connection {
            owner.take();
            EXPANDER_STATE.lock().await.as_mut().take();
            handle_power(pins, false).await;
            handle_set_cs(pins, 0).await;
        }
    }
}
