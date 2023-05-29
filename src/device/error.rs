use core::fmt;

use embassy_nrf::twim;
use embassy_sync::channel::TrySendError;
use thiserror_no_std::Error;

#[derive(Error, Debug)]
pub enum DeviceError {
    // #[error("data store disconnected")]
    // Disconnect(#[from] io::Error),

    #[error("Enum is out of boundaries")]
    EnumValueOutOfBoundaries,

    #[error("Format error")]
    FmtError(#[from] fmt::Error),

    #[error("Send debug error")]
    SendDebugError(#[from] TrySendError<[u8; 64]>),
}


#[derive(Error, Debug)]
pub enum CustomI2CError {
    #[error("Send debug error")]
    TwimError(#[from] twim::Error),
}


/// BME280 errors
#[derive(Error, Debug)]
pub enum Bme280Error {
    #[error("Failed to compensate a raw measurement")]
    CompensationFailed,

    #[error("I²C error")]
    Bus(#[from] CustomI2CError),

    #[error("Failed to parse sensor data")]
    InvalidData,

    #[error("No calibration data is available (probably forgot to call or check BME280::init for failure)")]
    NoCalibrationData,

    #[error("Chip ID doesn't match expected value")]
    UnsupportedChip,

    #[error("Delay error")]
    Delay,
}