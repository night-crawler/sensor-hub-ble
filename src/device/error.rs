use core::fmt;

use defmt::error;
use embassy_nrf::{spim, twim};
use embassy_sync::channel::TrySendError;
use nrf_softdevice::ble::gatt_server;
use thiserror_no_std::Error;

use crate::common::device::config::BLE_DEBUG_ARRAY_LEN;

#[derive(Error, Debug)]
pub enum DeviceError {
    #[error("Format error")]
    FmtError(#[from] fmt::Error),

    #[error("Spawn error")]
    SpawnError(#[from] embassy_executor::SpawnError),

    #[error("Send debug error")]
    SendDebugError(#[from] TrySendError<[u8; BLE_DEBUG_ARRAY_LEN]>),

    #[error("Flash error {0}")]
    Flash(#[from] FlashManagerError),

    #[error("GATT SetValueError")]
    SetValueError(#[from] gatt_server::SetValueError),
}

#[derive(Error, Debug, defmt::Format)]
pub enum FlashManagerError {
    #[error("Flash Error: {0:?}")]
    FlashError(#[from] nrf_softdevice::FlashError),

    #[error("Race condition: {0}, {1}")]
    RaceCondition(usize, usize),
}


#[derive(Error, Debug)]
pub enum CustomI2CError {
    #[error("I2C error")]
    TwimError(#[from] twim::Error),
}

#[derive(Error, Debug, defmt::Format)]
pub(crate) enum ExpanderError {
    #[error("Invalid command")]
    MutexReleaseNotLocked,

    #[error("Mutex not locked")]
    MutexNotLocked,

    #[error("Mutex locked twice by same client")]
    MutexAcquireTwiceSameClient,

    #[error("Mutex locked by other client")]
    MutexAcquiredByOtherClient,

    #[error("Invalid command")]
    InvalidCommand(u8),

    #[error("GATT GetValueError")]
    GetValueError(#[from] gatt_server::GetValueError),

    #[error("GATT SetValueError")]
    SetValueError(#[from] gatt_server::SetValueError),

    #[error("GATT SPI Expander Error")]
    SpiError(#[from] spim::Error),

    #[error("GATT I2C Expander Error")]
    I2cError(#[from] twim::Error),

    #[error("Invalid expander type {0}")]
    InvalidExpanderType(u8),

    #[error("Invalid CS {0}")]
    InvalidCs(u8),

    #[error("Invalid size read: {0}")]
    InvalidSizeRead(u16),

    #[error("Invalid size write: {0}")]
    InvalidSizeWrite(u16),

    #[error("Incomplete data")]
    IncompleteData,

    #[error("State not initialized")]
    StateNotInitialized,

    #[error("Timeout")]
    Timeout,
}
