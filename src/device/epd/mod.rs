use defmt::info;
use embassy_time::{Duration, Timer};

use crate::common::device::epd::constants::{LUT_FULL_UPDATE, LUT_PARTIAL_UPDATE};
use crate::common::device::epd::interface::DisplayInterface;
use crate::common::device::error::CustomSpimError;

use self::color::Color;
use self::command::{
    Command,
    DeepSleepMode,
};

pub(crate) mod color;
pub(crate) mod command;
pub(crate) mod constants;
pub(crate) mod interface;
pub(crate) mod epd_controls;
pub(crate) mod img;
pub(crate) mod traits;

/// Width of the display.
pub const WIDTH: u32 = 122;

/// Height of the display
pub const HEIGHT: u32 = 250;

const IS_BUSY_LOW: bool = false;

/// Epd2in13 (V2) driver
pub struct Epd2in13<I: DisplayInterface> {
    /// Connection Interface
    interface: I,

    sleep_mode: DeepSleepMode,

    /// Background Color
    background_color: Color,
}

impl<I: DisplayInterface> Epd2in13<I> {
    pub fn new(interface: I) -> Self {
        Self {
            interface,
            sleep_mode: DeepSleepMode::Mode2,
            background_color: Color::White,
        }
    }
    async fn send_command(&mut self, command: Command) -> Result<(), CustomSpimError> {
        info!("Executing command {:?}", command);
        self.interface.send_command(command).await?;
        info!("Executed command {:?}: DONE", command);
        Ok(())
    }

    async fn send_data(&mut self, data: &[u8]) -> Result<(), CustomSpimError> {
        info!("Sending data");
        self.interface.send_data(data).await?;
        info!("Data sent");
        Ok(())
    }

    async fn send_command_with_data(
        &mut self,
        command: Command,
        data: &[u8],
    ) -> Result<(), CustomSpimError> {
        info!("Executing command with data {:?}", command);
        self.interface.send_command_with_data(command, data).await?;
        info!("Executed command with data {:?}: DONE", command);
        Ok(())
    }

    pub async fn wait_until_idle(&mut self) {
        self.interface.wait_until_idle(IS_BUSY_LOW).await;
    }

    pub async fn reset(&mut self) -> Result<(), CustomSpimError> {
        self.interface.reset(0, 0).await;
        self.wait_until_idle().await;
        Ok(())
    }

    pub async fn turn_on_display(&mut self) -> Result<(), CustomSpimError> {
        self.send_command_with_data(Command::DisplayUpdateControl2, &[0xC7]).await?;
        self.send_command(Command::MasterActivation).await?;
        self.wait_until_idle().await;
        Ok(())
    }

    pub async fn turn_on_display_part(&mut self) -> Result<(), CustomSpimError> {
        self.send_command_with_data(Command::DisplayUpdateControl2, &[0x0f]).await?;
        self.send_command(Command::MasterActivation).await?;
        self.wait_until_idle().await;
        Ok(())
    }

    pub async fn lut(&mut self, data: &[u8]) -> Result<(), CustomSpimError> {
        self.send_command_with_data(Command::WriteLutRegister, data).await?;
        self.wait_until_idle().await;
        Ok(())
    }

    pub async fn set_lut(&mut self, lut: &[u8]) -> Result<(), CustomSpimError> {
        self.lut(lut).await?;

        // 0x22,0x17,0x41,0x00,0x32,0x36,
        self.send_command_with_data(Command::Unknown1, &[0x22]).await?;
        self.send_command_with_data(Command::GateDrivingVoltageCtrl, &[0x17]).await?;
        self.send_command_with_data(Command::SourceDrivingVoltageCtrl, &[0x41, 0x0, 0x32]).await?;
        self.send_command_with_data(Command::WriteVcomRegister, &[0x78]).await?;

        Ok(())
    }

    pub async fn set_window(&mut self, x_start: u32, y_start: u32, x_end: u32, y_end: u32) -> Result<(), CustomSpimError> {
        self.send_command_with_data(Command::SetRamXAddressStartEndPosition, &[
            ((x_start >> 3) & 0xFF) as u8,
            ((x_end >> 3) & 0xFF) as u8
        ]).await?;

        self.send_command_with_data(Command::SetRamYAddressStartEndPosition, &[
            (y_start & 0xFF) as u8,
            ((y_start >> 8) & 0xFF) as u8,
            (y_end & 0xFF) as u8,
            ((y_end >> 8) & 0xFF) as u8,
        ]).await?;

        Ok(())
    }

    pub async fn set_cursor(&mut self, x: u32, y: u32) -> Result<(), CustomSpimError> {
        self.send_command_with_data(Command::SetRamXAddressCounter, &[(x & 0xFF) as u8]).await?;

        self.send_command_with_data(Command::SetRamYAddressCounter, &[
            (y & 0xFF) as u8,
            ((y >> 8) & 0xFF) as u8
        ]).await?;

        Ok(())
    }

    pub async fn init(&mut self) -> Result<(), CustomSpimError> {
        self.reset().await?;

        self.wait_until_idle().await;
        self.send_command(Command::SwReset).await?;
        self.wait_until_idle().await;
        Timer::after(Duration::from_millis(10)).await;

        self.send_command_with_data(Command::DriverOutputControl, &[
            0xf9,
            0x00,
            0x00,
        ]).await?;


        self.send_command_with_data(Command::DataEntryModeSetting, &[0x03]).await?;

        self.set_window(0, 0, WIDTH - 1, HEIGHT - 1).await?;
        self.set_cursor(0, 0).await?;

        self.send_command_with_data(Command::BorderWaveformControl, &[0x05]).await?;
        self.send_command_with_data(Command::DisplayUpdateControl1, &[0x00, 0x80]).await?;
        self.send_command_with_data(Command::UnknownTempSensor, &[0x80]).await?;

        self.wait_until_idle().await;

        self.set_lut(&LUT_FULL_UPDATE).await?;

        Ok(())
    }

    pub async fn display(&mut self, image: &[u8]) -> Result<(), CustomSpimError> {
        self.send_command_with_data(Command::WriteRam, image).await?;
        // self.set_window(0, 0, WIDTH - 1, HEIGHT - 1).await?;
        // self.set_cursor(0, 0).await?;

        self.turn_on_display().await?;


        Ok(())
    }

    pub async fn display_partial(&mut self, image: &[u8]) -> Result<(), CustomSpimError> {
        self.reset().await?;

        self.set_lut(&LUT_PARTIAL_UPDATE).await?;

        self.send_command_with_data(Command::WriteOtpSelection, &[
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x40,
            0x00,
            0x00,
            0x00,
            0x00,
        ]).await?;

        self.send_command_with_data(Command::BorderWaveformControl, &[0x80]).await?;
        self.send_command_with_data(Command::DisplayUpdateControl2, &[0xC0]).await?;
        self.send_command(Command::MasterActivation).await?;

        self.wait_until_idle().await;

        self.set_window(0, 0, WIDTH - 1, HEIGHT - 1).await?;
        self.set_cursor(0, 0).await?;

        self.send_command_with_data(Command::WriteRam, image).await?;
        self.turn_on_display_part().await?;

        Ok(())
    }

    pub async fn display_part_base_image(&mut self, image: &[u8]) -> Result<(), CustomSpimError> {
        self.send_command_with_data(Command::WriteRam, image).await?;

        self.send_command_with_data(Command::WriteRamRed, image).await?;
        self.turn_on_display().await?;

        Ok(())
    }

    pub async fn clear(&mut self, color: Color) -> Result<(), CustomSpimError> {
        self.send_command(Command::WriteRam).await?;
        self.interface.send_data_x_times(color.get_byte_value(), 4000).await?;
        self.turn_on_display().await?;
        Ok(())
    }

    pub async fn sleep(&mut self) -> Result<(), CustomSpimError> {
        self.send_command_with_data(Command::DeepSleepMode, &[0x01]).await?;
        Ok(())
    }
}



