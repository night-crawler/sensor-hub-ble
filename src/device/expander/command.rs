use crate::common::device::error::ExpanderError;

#[derive(defmt::Format, Debug, Copy, Clone)]
pub(crate) enum Command {
    Write,
    Read,
    Transfer,
    I2cScan
}

impl TryFrom<u8> for Command {
    type Error = ExpanderError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Command::Write),
            0x01 => Ok(Command::Read),
            0x02 => Ok(Command::Transfer),
            0x03 => Ok(Command::I2cScan),
            _ => Err(ExpanderError::InvalidCommand(value))
        }
    }
}
