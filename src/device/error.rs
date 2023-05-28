use core::fmt;

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
