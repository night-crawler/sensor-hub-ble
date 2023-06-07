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
use crate::common::device::epd::interface::DisplayInterface;
use crate::common::device::epd::traits::{InternalWiAdditions, RefreshLut, WaveshareDisplay};
use crate::common::device::error::CustomSpimError;

use self::color::Color;
use self::command::{
    BorderWaveForm, BorderWaveFormFixLevel, BorderWaveFormGs, BorderWaveFormVbd, Command,
    DataEntryModeDir, DataEntryModeIncr, DeepSleepMode, DisplayUpdateControl2, DriverOutput,
    GateDrivingVoltage, I32Ext, SourceDrivingVoltage, Vcom,
};
use self::constants::{LUT_FULL_UPDATE, LUT_PARTIAL_UPDATE};

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
            sleep_mode: DeepSleepMode::Normal,
            background_color: DEFAULT_BACKGROUND_COLOR,
            refresh: RefreshLut::Full,
        }
    }
}

impl<I: DisplayInterface> InternalWiAdditions for Epd2in13<I>
{
    async fn init(&mut self) -> Result<(), CustomSpimError> {
        // HW reset
        self.interface.reset(2000, 10_000).await;

        if self.refresh == RefreshLut::Quick {
            self.set_vcom_register((-9).vcom()).await?;
            self.wait_until_idle().await?;

            self.set_lut(Some(self.refresh)).await?;

            // Python code does this, not sure why
            // self.cmd_with_data(spi, Command::WriteOtpSelection, &[0, 0, 0, 0, 0x40, 0, 0]).await?;

            // During partial update, clock/analog are not disabled between 2
            // updates.
            self.set_display_update_control_2(
                DisplayUpdateControl2::new().enable_analog().enable_clock(),
            ).await?;
            self.command(Command::MasterActivation).await?;
            self.wait_until_idle().await?;

            self.set_border_waveform(
                BorderWaveForm {
                    vbd: BorderWaveFormVbd::Gs,
                    fix_level: BorderWaveFormFixLevel::Vss,
                    gs_trans: BorderWaveFormGs::Lut1,
                },
            ).await?;
        } else {
            info!("Initializing display (refresh = full)");
            self.wait_until_idle().await?;
            info!("Display is idle");
            self.command(Command::SwReset).await?;
            info!("Reset done");
            self.wait_until_idle().await?;
            info!("Display is idle");

            self.set_driver_output(
                DriverOutput {
                    scan_is_linear: true,
                    scan_g0_is_first: true,
                    scan_dir_incr: true,
                    width: (WIDTH - 1) as u16,
                },
            ).await?;
            info!("Setting driver output done");

            // These 2 are the reset values
            // self.set_dummy_line_period(0x30).await?;
            // info!("Setting dummy line period done");

            self.set_gate_scan_start_position(0).await?;
            info!("Setting gate scan start position done");

            self.set_data_entry_mode(DataEntryModeIncr::XIncrYIncr, DataEntryModeDir::XDir).await?;
            info!("Setting data entry mode done");

            // Use simple X/Y auto increase
            self.set_ram_area(0, 0, WIDTH - 1, HEIGHT- 1).await?;
            info!("Setting ram area done");

            self.set_ram_address_counters(0, 0).await?;
            info!("set_ram_address_counters done");

            self.set_border_waveform(
                BorderWaveForm {
                    vbd: BorderWaveFormVbd::Gs,
                    fix_level: BorderWaveFormFixLevel::Vss,
                    gs_trans: BorderWaveFormGs::Lut3,
                },
            ).await?;
            info!("Setting border waveform done");

            self.set_vcom_register((-21).vcom()).await?;

            self.set_gate_driving_voltage(190.gate_driving_decivolt()).await?;
            self.set_source_driving_voltage(
                150.source_driving_decivolt(),
                50.source_driving_decivolt(),
                (-150).source_driving_decivolt(),
            ).await?;

            self.set_gate_line_width(10).await?;

            self.set_lut(Some(self.refresh)).await?;
        }

        info!("Waiting for idle...");
        self.wait_until_idle().await?;
        Ok(())
    }
}

impl<I: DisplayInterface> WaveshareDisplay for Epd2in13<I>
{
    type DisplayColor = Color;

    async fn wake_up(&mut self) -> Result<(), CustomSpimError> {
        self.init().await
    }

    async fn sleep(&mut self) -> Result<(), CustomSpimError> {
        self.wait_until_idle().await?;

        // All sample code enables and disables analog/clocks...
        self.set_display_update_control_2(
            DisplayUpdateControl2::new()
                .enable_analog()
                .enable_clock()
                .disable_analog()
                .disable_clock(),
        ).await?;
        self.command(Command::MasterActivation).await?;

        self.set_sleep_mode(self.sleep_mode).await?;
        Ok(())
    }

    async fn update_frame(&mut self, buffer: &[u8]) -> Result<(), CustomSpimError> {
        assert_eq!(buffer.len(), buffer_len(WIDTH as usize, HEIGHT as usize));
        self.set_ram_area(0, 0, WIDTH - 1, HEIGHT - 1).await?;
        self.set_ram_address_counters(0, 0).await?;

        self.cmd_with_data(Command::WriteRam, buffer).await?;

        if self.refresh == RefreshLut::Full {
            // Always keep the base buffer equal to current if not doing partial refresh.
            self.set_ram_area(0, 0, WIDTH - 1, HEIGHT - 1).await?;
            self.set_ram_address_counters(0, 0).await?;

            self.cmd_with_data(Command::WriteRamRed, buffer).await?;
        }
        Ok(())
    }

    /// Updating only a part of the frame is not supported when using the
    /// partial refresh feature. The function will panic if called when set to
    /// use partial refresh.
    async fn update_partial_frame(
        &mut self,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), CustomSpimError> {
        assert_eq!((width * height / 8) as usize, buffer.len());

        // This should not be used when doing partial refresh. The RAM_RED must
        // be updated with the last buffer having been displayed. Doing partial
        // update directly in RAM makes this update impossible (we can't read
        // RAM content). Using this function will most probably make the actual
        // display incorrect as the controler will compare with something
        // incorrect.
        assert_eq!(self.refresh, RefreshLut::Full);

        self.set_ram_area(x, y, x + width, y + height).await?;
        self.set_ram_address_counters(x, y).await?;

        self.cmd_with_data(Command::WriteRam, buffer).await?;

        if self.refresh == RefreshLut::Full {
            // Always keep the base buffer equals to current if not doing partial refresh.
            self.set_ram_area(x, y, x + width, y + height).await?;
            self.set_ram_address_counters(x, y).await?;

            self.cmd_with_data(Command::WriteRamRed, buffer).await?;
        }

        Ok(())
    }

    /// Never use directly this function when using partial refresh, or also
    /// keep the base buffer in syncd using `set_partial_base_buffer` function.
    async fn display_frame(&mut self) -> Result<(), CustomSpimError> {
        if self.refresh == RefreshLut::Full {
            self.set_display_update_control_2(
                DisplayUpdateControl2::new()
                    .enable_clock()
                    .enable_analog()
                    .display()
                    .disable_analog()
                    .disable_clock(),
            ).await?;
        } else {
            self.set_display_update_control_2(DisplayUpdateControl2::new().display()).await?;
        }
        self.command(Command::MasterActivation).await?;
        self.wait_until_idle().await?;

        Ok(())
    }

    async fn update_and_display_frame(&mut self, buffer: &[u8]) -> Result<(), CustomSpimError> {
        self.update_frame(buffer).await?;
        self.display_frame().await?;

        if self.refresh == RefreshLut::Quick {
            self.set_partial_base_buffer(buffer).await?;
        }
        Ok(())
    }

    async fn clear_frame(&mut self) -> Result<(), CustomSpimError> {
        info!("Clear frame");
        let color = self.background_color.get_byte_value();

        self.set_ram_area(0, 0, WIDTH - 1, HEIGHT - 1).await?;
        info!("Clear frame: set_ram_area succeeded");

        self.set_ram_address_counters(0, 0).await?;
        info!("Clear frame: set_ram_area and set_ram_address_counters succeeded");

        self.command(Command::WriteRam).await?;
        info!("Clear frame: WriteRam succeeded");
        self.interface.data_x_times(
            color,
            buffer_len(WIDTH as usize, HEIGHT as usize) as u32,
        ).await?;
        info!("Clear frame: data_x_times succeeded");

        // Always keep the base buffer equals to current if not doing partial refresh.
        if self.refresh == RefreshLut::Full {
            self.set_ram_area(0, 0, WIDTH - 1, HEIGHT - 1).await?;
            self.set_ram_address_counters(0, 0).await?;

            self.command(Command::WriteRamRed).await?;
            self.interface.data_x_times(
                color,
                buffer_len(WIDTH as usize, HEIGHT as usize) as u32,
            ).await?;
        }
        Ok(())
    }

    async fn set_background_color(&mut self, background_color: Color) {
        self.background_color = background_color;
    }

    async fn background_color(&self) -> &Color {
        &self.background_color
    }

    fn width(&self) -> u32 {
        WIDTH
    }

    fn height(&self) -> u32 {
        HEIGHT
    }

    async fn set_lut(
        &mut self,
        refresh_rate: Option<RefreshLut>,
    ) -> Result<(), CustomSpimError> {
        let buffer = match refresh_rate {
            Some(RefreshLut::Full) | None => &LUT_FULL_UPDATE,
            Some(RefreshLut::Quick) => &LUT_PARTIAL_UPDATE,
        };

        self.cmd_with_data(Command::WriteLutRegister, buffer).await
    }

    async fn wait_until_idle(&mut self) -> Result<(), CustomSpimError> {
        self.interface.wait_until_idle(IS_BUSY_LOW).await;
        Ok(())
    }
}

impl<I: DisplayInterface> Epd2in13<I> {
    /// When using partial refresh, the controller uses the provided buffer for
    /// comparison with new buffer.
    pub async fn set_partial_base_buffer(
        &mut self,
        buffer: &[u8],
    ) -> Result<(), CustomSpimError> {
        assert_eq!(buffer_len(WIDTH as usize, HEIGHT as usize), buffer.len());
        self.set_ram_area(0, 0, WIDTH - 1, HEIGHT - 1).await?;
        self.set_ram_address_counters(0, 0).await?;

        self.cmd_with_data(Command::WriteRamRed, buffer).await?;
        Ok(())
    }

    /// Selects which sleep mode will be used when triggering the deep sleep.
    pub async fn set_deep_sleep_mode(&mut self, mode: DeepSleepMode) {
        self.sleep_mode = mode;
    }

    /// Sets the refresh mode. When changing mode, the screen will be
    /// re-initialized accordingly.
    pub async fn set_refresh(
        &mut self,
        refresh: RefreshLut,
    ) -> Result<(), CustomSpimError> {
        if self.refresh != refresh {
            self.refresh = refresh;
            self.init().await?;
        }
        Ok(())
    }

    async fn set_gate_scan_start_position(
        &mut self,
        start: u16,
    ) -> Result<(), CustomSpimError> {
        assert!(start <= 295);
        self.cmd_with_data(
            Command::GateScanStartPosition,
            &[(start & 0xFF) as u8, ((start >> 8) & 0x1) as u8],
        ).await
    }

    async fn set_border_waveform(
        &mut self,
        borderwaveform: BorderWaveForm,
    ) -> Result<(), CustomSpimError> {
        self.cmd_with_data(
            Command::BorderWaveformControl,
            &[borderwaveform.to_u8()],
        ).await
    }

    async fn set_vcom_register(&mut self, vcom: Vcom) -> Result<(), CustomSpimError> {
        self.cmd_with_data(Command::WriteVcomRegister, &[vcom.0]).await
    }

    async fn set_gate_driving_voltage(
        &mut self,
        voltage: GateDrivingVoltage,
    ) -> Result<(), CustomSpimError> {
        self.cmd_with_data(Command::GateDrivingVoltageCtrl, &[voltage.0]).await
    }

    async fn set_dummy_line_period(
        &mut self,
        number_of_lines: u8,
    ) -> Result<(), CustomSpimError> {
        assert!(number_of_lines <= 127);
        self.cmd_with_data(Command::SetDummyLinePeriod, &[number_of_lines]).await
    }

    async fn set_gate_line_width(&mut self, width: u8) -> Result<(), CustomSpimError> {
        self.cmd_with_data(Command::SetGateLineWidth, &[width & 0x0F]).await
    }

    /// Sets the source driving voltage value
    async fn set_source_driving_voltage(
        &mut self,
        vsh1: SourceDrivingVoltage,
        vsh2: SourceDrivingVoltage,
        vsl: SourceDrivingVoltage,
    ) -> Result<(), CustomSpimError> {
        self.cmd_with_data(
            Command::SourceDrivingVoltageCtrl,
            &[vsh1.0, vsh2.0, vsl.0],
        ).await
    }

    /// Prepare the actions that the next master activation command will
    /// trigger.
    async fn set_display_update_control_2(
        &mut self,
        value: DisplayUpdateControl2,
    ) -> Result<(), CustomSpimError> {
        self.cmd_with_data(Command::DisplayUpdateControl2, &[value.0]).await
    }

    /// Triggers the deep sleep mode
    async fn set_sleep_mode(&mut self, mode: DeepSleepMode) -> Result<(), CustomSpimError> {
        self.cmd_with_data(Command::DeepSleepMode, &[mode as u8]).await
    }

    async fn set_driver_output(&mut self, output: DriverOutput) -> Result<(), CustomSpimError> {
        self.cmd_with_data(Command::DriverOutputControl, &output.to_bytes()).await
    }

    /// Sets the data entry mode (ie. how X and Y positions changes when writing
    /// data to RAM)
    async fn set_data_entry_mode(
        &mut self,
        counter_incr_mode: DataEntryModeIncr,
        counter_direction: DataEntryModeDir,
    ) -> Result<(), CustomSpimError> {
        let mode = counter_incr_mode as u8 | counter_direction as u8;
        self.cmd_with_data(Command::DataEntryModeSetting, &[mode]).await
    }

    /// Sets both X and Y pixels ranges
    async fn set_ram_area(
        &mut self,
        start_x: u32,
        start_y: u32,
        end_x: u32,
        end_y: u32,
    ) -> Result<(), CustomSpimError> {
        self.cmd_with_data(
            Command::SetRamXAddressStartEndPosition,
            &[(start_x >> 3) as u8, (end_x >> 3) as u8],
        ).await?;

        self.cmd_with_data(
            Command::SetRamYAddressStartEndPosition,
            &[
                start_y as u8,
                (start_y >> 8) as u8,
                end_y as u8,
                (end_y >> 8) as u8,
            ],
        ).await
    }

    /// Sets both X and Y pixels counters when writing data to RAM
    async fn set_ram_address_counters(
        &mut self,
        x: u32,
        y: u32,
    ) -> Result<(), CustomSpimError> {
        self.wait_until_idle().await?;
        self.cmd_with_data(Command::SetRamXAddressCounter, &[(x >> 3) as u8]).await?;

        self.cmd_with_data(
            Command::SetRamYAddressCounter,
            &[y as u8, (y >> 8) as u8],
        ).await?;
        Ok(())
    }

    async fn command(&mut self, command: Command) -> Result<(), CustomSpimError> {
        self.interface.cmd(command).await?;
        self.wait_until_idle().await?;
        Ok(())
    }

    async fn cmd_with_data(
        &mut self,
        command: Command,
        data: &[u8],
    ) -> Result<(), CustomSpimError> {
        self.interface.cmd_with_data(command, data).await?;
        self.wait_until_idle().await?;
        Ok(())
    }
}
