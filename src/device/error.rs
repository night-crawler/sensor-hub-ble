use core::fmt;

use defmt::error;
use embassy_nrf::{spim, twim};
use embassy_sync::channel::TrySendError;
use thiserror_no_std::Error;
use crate::common::device::config::BLE_DEBUG_ARRAY_LEN;

#[derive(Error, Debug)]
pub enum DeviceError {
    #[error("Format error {0}")]
    FmtError(#[from] fmt::Error),

    #[error("Spawn error")]
    SpawnError(#[from] embassy_executor::SpawnError),

    #[error("Send debug error")]
    SendDebugError(#[from] TrySendError<[u8; BLE_DEBUG_ARRAY_LEN]>),
}

#[derive(Error, Debug)]
pub enum CustomI2CError {
    #[error("I2C error")]
    TwimError(#[from] twim::Error),
}

#[derive(Error, Debug, defmt::Format)]
pub enum CustomSpimError {
    #[error("SPI Error")]
    SpimError(#[from] spim::Error),

    #[error("Bitbang")]
    BitbangSpimError(#[from] crate::common::bitbang::spi::SpiBbError),
}
