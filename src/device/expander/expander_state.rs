use embassy_nrf::peripherals;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Duration;
use embassy_time::Timer;
use futures::FutureExt;
use futures::select_biased;
use rclite::Arc;

use crate::common::device::config::{BLE_EXPANDER_BUF_SIZE, BLE_EXPANDER_CONTROL_BYTES_SIZE, BLE_EXPANDER_EXEC_TIMEOUT};
use crate::common::device::peripherals_manager::ExpanderPins;
use crate::common::device::error::ExpanderError;
use crate::common::device::expander::{handle_i2c_exec, handle_spi_exec};
use crate::common::device::expander::command::Command;

#[derive(Debug, defmt::Format, Copy, Clone)]
pub(crate) enum ExpanderType {
    NotSet,
    Spi,
    I2c,
}

impl TryFrom<u8> for ExpanderType {
    type Error = ExpanderError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::NotSet),
            1 => Ok(Self::Spi),
            2 => Ok(Self::I2c),
            _ => Err(ExpanderError::InvalidExpanderType(value)),
        }
    }
}

#[derive(Default, defmt::Format)]
pub(crate) struct ExpanderFlags {
    pub(crate) expander_lock_type: Option<ExpanderType>,
    pub(crate) power: Option<bool>,
    pub(crate) power_wait_duration: Duration,
    pub(crate) cs: Option<u8>,
    pub(crate) cs_wait_duration: Duration,
    pub(crate) command: Option<Command>,
    pub(crate) address: Option<u8>,
    pub(crate) size_read: Option<usize>,
    pub(crate) size_write: Option<usize>,
    pub(crate) has_mosi: bool,
}

impl TryFrom<&[u8]> for ExpanderFlags {
    type Error = ExpanderError;

    /// [
    ///     [0] control_bits,
    ///     [1] reserved_control_bits,
    ///     [2] lock_type,
    ///     [3] power_on_off,
    ///     [4] power_wait,
    ///     [5] cs,
    ///     [6] cs_wait,
    ///     [7] command,
    ///     [8] address,
    ///     [9,10] [size_read, size_read],
    ///     [11, 12] [size_write, size_write],
    ///     [13] reserved,
    ///     [14] reserved,
    ///     [15] reserved,
    ///     ..mosi
    /// ]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let has_lock = value[0] & 0b1000_0000 != 0;
        let has_power = value[0] & 0b0100_0000 != 0;
        let has_cs = value[0] & 0b0010_0000 != 0;
        let has_command = value[0] & 0b0001_0000 != 0;
        let has_address = value[0] & 0b0000_1000 != 0;
        let has_size_read = value[0] & 0b0000_0100 != 0;
        let has_size_write = value[0] & 0b0000_0010 != 0;
        let has_mosi = value[0] & 0b0000_0001 != 0;


        let mut instance = Self {
            power_wait_duration: Duration::from_millis((value[4] as u64) * 10),
            cs_wait_duration: Duration::from_millis((value[6] as u64) * 10),
            ..Default::default()
        };

        if has_lock {
            instance.expander_lock_type = Some(ExpanderType::try_from(value[2])?);
        }
        if has_power {
            instance.power = Some(value[3] != 0);
        }
        if has_cs {
            let cs = value[5];
            if cs > 7 {
                return Err(ExpanderError::InvalidCs(cs));
            }
            instance.cs = Some(cs);
        }

        if has_command {
            instance.command = Some(Command::try_from(value[7])?);
        }
        if has_address {
            instance.address = Some(value[8]);
        }
        if has_size_read {
            let size = u16::from_le_bytes([value[9], value[10]]);
            if size > BLE_EXPANDER_BUF_SIZE as u16 {
                return Err(ExpanderError::InvalidSizeRead(size));
            }
            instance.size_read = Some(size as usize);
        }
        if has_size_write {
            let size = u16::from_le_bytes([value[11], value[12]]);
            if size > BLE_EXPANDER_BUF_SIZE as u16 {
                return Err(ExpanderError::InvalidSizeWrite(size));
            }
            instance.size_write = Some(size as usize);
        }

        instance.has_mosi = has_mosi;

        Ok(instance)
    }
}

pub(crate) struct ExpanderState {
    pub(crate) mosi: [u8; BLE_EXPANDER_BUF_SIZE],
    pub(crate) flags: ExpanderFlags,
    pub(crate) timeout: Duration,
}

impl Default for ExpanderState {
    fn default() -> Self {
        Self {
            mosi: [0u8; BLE_EXPANDER_BUF_SIZE],
            flags: ExpanderFlags::default(),
            timeout: BLE_EXPANDER_EXEC_TIMEOUT,
        }
    }
}


impl ExpanderFlags {
    pub(crate) fn update(&mut self, next_flags: &ExpanderFlags) {
        if let Some(lock) = next_flags.expander_lock_type {
            self.expander_lock_type = Some(lock);
        }
        if let Some(power) = next_flags.power {
            self.power = Some(power);
        }
        if let Some(cs) = next_flags.cs {
            self.cs = Some(cs);
        }
        if let Some(command) = next_flags.command {
            self.command = Some(command);
        }
        if let Some(address) = next_flags.address {
            self.address = Some(address);
        }
        if let Some(size_read) = next_flags.size_read {
            self.size_read = Some(size_read);
        }
        if let Some(size_write) = next_flags.size_write {
            self.size_write = Some(size_write);
        }
        self.power_wait_duration = next_flags.power_wait_duration;
        self.cs_wait_duration = next_flags.cs_wait_duration;
        self.has_mosi = next_flags.has_mosi;
    }
}

impl ExpanderState {
    pub(crate) fn default_with_type(expander_type: ExpanderType) -> Self {
        Self {
            mosi: [0u8; BLE_EXPANDER_BUF_SIZE],
            flags: ExpanderFlags {
                expander_lock_type: Some(expander_type),
                ..Default::default()
            },
            timeout: BLE_EXPANDER_EXEC_TIMEOUT,
        }
    }

    pub(crate) fn update(&mut self, data_bundle: [u8; BLE_EXPANDER_BUF_SIZE + BLE_EXPANDER_CONTROL_BYTES_SIZE]) -> Result<(), ExpanderError> {
        let controls = &data_bundle[..BLE_EXPANDER_CONTROL_BYTES_SIZE];
        let next_flags = ExpanderFlags::try_from(controls)?;
        let mosi = &data_bundle[BLE_EXPANDER_CONTROL_BYTES_SIZE..];
        self.flags.update(&next_flags);

        if self.flags.has_mosi {
            self.mosi.copy_from_slice(mosi);
        }

        Ok(())
    }

    pub(crate) async fn exec(
        &self,
        pins: &Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
    ) -> Result<Option<[u8; BLE_EXPANDER_BUF_SIZE]>, ExpanderError> {
        select_biased! {
            response = self.exec_internal(pins).fuse() => {
                response
            }
            _ = Timer::after(self.timeout).fuse() => {
                Err(ExpanderError::Timeout)
            }
        }
    }
    async fn exec_internal(
        &self,
        pins: &Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
    ) -> Result<Option<[u8; BLE_EXPANDER_BUF_SIZE]>, ExpanderError> {
        match self.flags.expander_lock_type {
            None | Some(ExpanderType::NotSet) => Err(ExpanderError::MutexNotLocked),
            Some(ExpanderType::Spi) => {
                match (self.flags.command, self.flags.size_write) {
                    (Some(command), Some(size_write)) => {
                        handle_spi_exec(&pins, command, &self.mosi[..size_write]).await
                    }
                    _ => Err(ExpanderError::IncompleteData)
                }
            }
            Some(ExpanderType::I2c) => {
                match (self.flags.command, self.flags.address, self.flags.size_write, self.flags.size_read) {
                    (Some(command), Some(address), Some(size_write), Some(size_read)) => {
                        handle_i2c_exec(&pins, address, command, &self.mosi[..size_write], size_read).await
                    }
                    _ => Err(ExpanderError::IncompleteData)
                }
            }
        }
    }
}

impl TryFrom<[u8; BLE_EXPANDER_BUF_SIZE + BLE_EXPANDER_CONTROL_BYTES_SIZE]> for ExpanderState {
    type Error = ExpanderError;

    fn try_from(value: [u8; BLE_EXPANDER_BUF_SIZE + BLE_EXPANDER_CONTROL_BYTES_SIZE]) -> Result<Self, Self::Error> {
        let mut state = ExpanderState::default();
        state.update(value)?;
        Ok(state)
    }
}
