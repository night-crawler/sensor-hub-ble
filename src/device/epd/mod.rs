//! A Driver for the Waveshare 2.13" E-Ink Display (V2) via SPI
//!
//! # References
//!
//! - [Waveshare product page](https://www.waveshare.com/wiki/2.13inch_e-Paper_HAT)
//! - [Waveshare C driver](https://github.com/waveshare/e-Paper/blob/master/RaspberryPi%26JetsonNano/c/lib/e-Paper/EPD_2in13_V2.c)
//! - [Waveshare Python driver](https://github.com/waveshare/e-Paper/blob/master/RaspberryPi%26JetsonNano/python/lib/waveshare_epd/epd2in13_V2.py)
//! - [Controller Datasheet SS1780](http://www.e-paper-display.com/download_detail/downloadsId=682.html)
//!

use defmt::info;
use embassy_time::{Duration, Timer};
use futures::FutureExt;
use crate::common::device::epd::constants::{LUT_FULL_UPDATE, LUT_PARTIAL_UPDATE};
use crate::common::device::epd::interface::DisplayInterface;
use crate::common::device::epd::traits::{InternalWiAdditions, RefreshLut, WaveshareDisplay};
use crate::common::device::error::CustomSpimError;

use self::color::Color;
use self::command::{
    Command,
    DeepSleepMode,
    I32Ext,
};

pub(crate) mod color;
pub(crate) mod command;
pub(crate) mod constants;
pub(crate) mod traits;
pub(crate) mod interface;
pub(crate) mod epd_controls;
pub(crate) mod img;


pub const fn buffer_len(width: usize, height: usize) -> usize {
    (width + 7) / 8 * height
}


/// Width of the display.
pub const WIDTH: u32 = 122;

/// Height of the display
pub const HEIGHT: u32 = 250;

/// Default Background Color
pub const DEFAULT_BACKGROUND_COLOR: Color = Color::White;
const IS_BUSY_LOW: bool = false;

/// Epd2in13 (V2) driver
pub struct Epd2in13<I: DisplayInterface> {
    /// Connection Interface
    interface: I,

    sleep_mode: DeepSleepMode,

    /// Background Color
    background_color: Color,
    refresh: RefreshLut,
}

impl<I: DisplayInterface> Epd2in13<I> {
    pub fn new(interface: I) -> Self {
        Self {
            interface,
            sleep_mode: DeepSleepMode::Mode2,
            background_color: Color::White,
            refresh: RefreshLut::Full,
        }
    }
    async fn command(&mut self, command: Command) -> Result<(), CustomSpimError> {
        info!("Executing command {:?}", command);
        self.interface.cmd(command).await?;
        info!("Executed command {:?}: DONE", command);
        Ok(())
    }

    async fn data(&mut self, data: &[u8]) -> Result<(), CustomSpimError> {
        info!("Sending data");
        self.interface.data(data).await?;
        info!("Data sent");
        Ok(())
    }

    async fn cmd_with_data(
        &mut self,
        command: Command,
        data: &[u8],
    ) -> Result<(), CustomSpimError> {
        info!("Executing command with data {:?}", command);
        self.interface.cmd_with_data(command, data).await?;
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
        self.cmd_with_data(Command::DisplayUpdateControl2, &[0xC7]).await?;
        self.command(Command::MasterActivation).await?;
        self.wait_until_idle().await;
        Ok(())
    }

    pub async fn turn_on_display_part(&mut self) -> Result<(), CustomSpimError> {
        self.cmd_with_data(Command::DisplayUpdateControl2, &[0x0f]).await?;
        self.command(Command::MasterActivation).await?;
        self.wait_until_idle().await;
        Ok(())
    }

    pub async fn lut(&mut self, data: &[u8]) -> Result<(), CustomSpimError> {
        self.cmd_with_data(Command::WriteLutRegister, data).await?;
        self.wait_until_idle().await;
        Ok(())
    }

    pub async fn set_lut(&mut self, lut: &[u8]) -> Result<(), CustomSpimError> {
        self.lut(lut).await?;

        // 0x22,0x17,0x41,0x00,0x32,0x36,
        self.cmd_with_data(Command::Unknown1, &[0x22]).await?;
        self.cmd_with_data(Command::GateDrivingVoltageCtrl, &[0x17]).await?;
        self.cmd_with_data(Command::SourceDrivingVoltageCtrl, &[0x41, 0x0, 0x32]).await?;
        self.cmd_with_data(Command::WriteVcomRegister, &[0x78]).await?;

        Ok(())
    }

    pub async fn set_window(&mut self, x_start: u32, y_start: u32, x_end: u32, y_end: u32) -> Result<(), CustomSpimError> {
        self.cmd_with_data(Command::SetRamXAddressStartEndPosition, &[
            ((x_start >> 3) & 0xFF) as u8,
            ((x_end >> 3) & 0xFF) as u8
        ]).await?;

        self.cmd_with_data(Command::SetRamYAddressStartEndPosition, &[
            (y_start & 0xFF) as u8,
            ((y_start >> 8) & 0xFF) as u8,
            (y_end & 0xFF) as u8,
            ((y_end >> 8) & 0xFF) as u8,
        ]).await?;

        Ok(())
    }

    pub async fn set_cursor(&mut self, x: u32, y: u32) -> Result<(), CustomSpimError> {
        self.cmd_with_data(Command::SetRamXAddressCounter, &[(x & 0xFF) as u8]).await?;

        self.cmd_with_data(Command::SetRamYAddressCounter, &[
            (y & 0xFF) as u8,
            ((y >> 8) & 0xFF) as u8
        ]).await?;

        Ok(())
    }

    pub async fn init(&mut self) -> Result<(), CustomSpimError> {
        self.reset().await?;

        self.wait_until_idle().await;
        self.command(Command::SwReset).await?;
        self.wait_until_idle().await;
        Timer::after(Duration::from_millis(10)).await;

        self.cmd_with_data(Command::DriverOutputControl, &[
            0xf9,
            0x00,
            0x00,
        ]).await?;


        self.cmd_with_data(Command::DataEntryModeSetting, &[0x03]).await?;

        self.set_window(0, 0, WIDTH - 1, HEIGHT - 1).await?;
        self.set_cursor(0, 0).await?;

        self.cmd_with_data(Command::BorderWaveformControl, &[0x05]).await?;
        self.cmd_with_data(Command::DisplayUpdateControl1, &[0x00, 0x80]).await?;
        self.cmd_with_data(Command::UnknownTempSensor, &[0x80]).await?;

        self.wait_until_idle().await;

        self.set_lut(&LUT_FULL_UPDATE).await?;

        Ok(())
    }

    pub async fn display(&mut self, image: &[u8]) -> Result<(), CustomSpimError> {
        let linewidth = if WIDTH % 8 == 0 {
            WIDTH / 8
        } else {
            WIDTH / 8 + 1
        };

        Timer::after(Duration::from_millis(100)).await;

        self.command(Command::WriteRam).await?;

        let mut counter = 0;

        self.set_window(0, 0, WIDTH - 1, HEIGHT - 1).await?;
        self.set_cursor(0, 0).await?;

        for j in 0..HEIGHT {
            for i in 0..linewidth {
                let index = (i + j * linewidth) as usize;
                let mut b = image[index];
                self.data(&[b]).await?;

                futures::select_biased! {
                    _ = self.wait_until_idle().fuse() => {
                        info!("Byte {} / {} sent: {:#04x}!", index, counter, b)
                    }
                    _ = Timer::after(Duration::from_millis(100)).fuse() => {
                        info!("Byte {} / {} - {:#04x} timeout!", index, counter, b)
                    }
                }
                counter += 1;
            }
        }

        self.turn_on_display().await?;


        Ok(())
    }

    pub async fn display_partial(&mut self, image: &[u8]) -> Result<(), CustomSpimError> {
        self.reset().await?;

        self.set_lut(&LUT_PARTIAL_UPDATE).await?;

        self.cmd_with_data(Command::WriteOtpSelection, &[
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

        self.cmd_with_data(Command::BorderWaveformControl, &[0x80]).await?;
        self.cmd_with_data(Command::DisplayUpdateControl2, &[0xC0]).await?;
        self.command(Command::MasterActivation).await?;

        self.wait_until_idle().await;

        self.set_window(0, 0, WIDTH - 1, HEIGHT - 1).await?;
        self.set_cursor(0, 0).await?;

        self.cmd_with_data(Command::WriteRam, image).await?;
        self.turn_on_display_part().await?;

        Ok(())
    }

    pub async fn display_part_base_image(&mut self, image: &[u8]) -> Result<(), CustomSpimError> {
        self.cmd_with_data(Command::WriteRam, image).await?;

        self.cmd_with_data(Command::WriteRamRed, image).await?;
        self.turn_on_display().await?;

        Ok(())
    }

    pub async fn clear(&mut self, color: Color) -> Result<(), CustomSpimError> {
        let linewidth = if WIDTH % 8 == 0 {
            WIDTH / 8
        } else {
            WIDTH / 8 + 1
        };


        self.command(Command::WriteRam).await?;
        for _ in 0..HEIGHT * linewidth {
            self.data(&[color.get_byte_value()]).await?;
        }
        self.turn_on_display().await?;
        Ok(())
    }

    pub async fn sleep(&mut self) -> Result<(), CustomSpimError> {
        self.cmd_with_data(Command::DeepSleepMode, &[0x01]).await?;
        Ok(())
    }
}



