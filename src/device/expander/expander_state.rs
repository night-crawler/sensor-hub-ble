use crate::common::device::config::BLE_EXPANDER_BUF_SIZE;
use crate::common::device::expander::command::Command;

#[derive(Debug, defmt::Format, Copy, Clone)]
pub(crate) enum ExpanderType {
    None,
    Spi,
    I2c,
}

pub(crate) struct ExpanderState {
    pub(crate) expander_type: ExpanderType,
    pub(crate) power: bool,
    pub(crate) size: usize,
    pub(crate) cs: u8,
    pub(crate) command: Command,
    pub(crate) mosi: [u8; BLE_EXPANDER_BUF_SIZE],
    pub(crate) i2c_address: u8,
}

impl ExpanderState {
    pub(crate) const fn new() -> Self {
        Self {
            expander_type: ExpanderType::Spi,
            power: false,
            size: 0,
            cs: 0,
            command: Command::Write,
            mosi: [0u8; BLE_EXPANDER_BUF_SIZE],
            i2c_address: 0,
        }
    }

    pub(crate) fn new_with_type(expander_type: ExpanderType) -> ExpanderState {
        let mut instance = Self::new();
        instance.expander_type = expander_type;
        instance
    }
}

