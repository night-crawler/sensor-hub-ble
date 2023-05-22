use thiserror_no_std::Error;

#[derive(Error, Debug)]
pub enum DeviceError {
    // #[error("data store disconnected")]
    // Disconnect(#[from] io::Error),

    #[error("Enum is out of boundaries")]
    EnumValueOutOfBoundaries
}
