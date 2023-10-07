use embassy_time::{Duration, Timer};

use embedded_hal_async::i2c;
use embedded_hal_async::i2c::ErrorType;
use num_traits::float::FloatCore;

/// All possible errors in this crate
#[derive(Debug)]
pub enum Error<E> {
    /// I²C bus error
    I2C(E),
}

/// Possible measurement modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeasurementMode {
    /// Automatic mode.
    ///
    /// Measurements are made continuously. The actual cadence depends on
    /// the integration time.
    Auto,
    /// Manual mode.
    ///
    /// Measurements are only triggered manually. See `trigger_measurement()`.
    /// This is also called "force mode" or "ActiveForce" mode.
    Manual,
}

/// Integration time
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntegrationTime {
    /// 40 ms
    _40ms,
    /// 80 ms
    _80ms,
    /// 160 ms
    _160ms,
    /// 320 ms
    _320ms,
    /// 640 ms
    _640ms,
    /// 1280 ms
    _1280ms,
}

impl IntegrationTime {
    pub fn sensitivity(&self) -> f32 {
        match self {
            IntegrationTime::_40ms => 0.25168,
            IntegrationTime::_80ms => 0.12584,
            IntegrationTime::_160ms => 0.06292,
            IntegrationTime::_320ms => 0.03146,
            IntegrationTime::_640ms => 0.01573,
            IntegrationTime::_1280ms => 0.007865,
        }
    }

    pub fn get_duration_ms(&self) -> u64 {
        match self {
            IntegrationTime::_40ms => 40,
            IntegrationTime::_80ms => 80,
            IntegrationTime::_160ms => 160,
            IntegrationTime::_320ms => 320,
            IntegrationTime::_640ms => 640,
            IntegrationTime::_1280ms => 1280,
        }
    }
}

/// Result of measurement of all channels
#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub struct AllChannelMeasurement {
    /// Red channel measurement.
    pub red: u16,
    /// Green channel measurement.
    pub green: u16,
    /// Blue channel measurement.
    pub blue: u16,
    /// White channel measurement.
    pub white: u16,
}

impl AllChannelMeasurement {
    pub fn compute_cct(&self) -> Option<f32> {
        // https://www.vishay.com/docs/84331/designingveml6040.pdf
        let (r, g, b) = (self.red as f32, self.green as f32, self.blue as f32);

        let corrected_color_x = (-0.023249 * r) + (0.291014 * g) + (-0.364880 * b);
        let corrected_color_y = (-0.042799 * r) + (0.272148 * g) + (-0.279591 * b);
        let corrected_color_z = (-0.155901 * r) + (0.251534 * g) + (-0.076240 * b);
        let color_total = corrected_color_x + corrected_color_y + corrected_color_z;

        if color_total < 0.001 {
            return None;
        }

        // Once the XYZ have been found, these can be used to derive the (x, y) coordinates,
        // which then denote a specific color, as depicted on the axes CIE color gamut on page 7.
        // For this the following equations can be used:
        let color_x = corrected_color_x / color_total;
        let color_y = corrected_color_y / color_total;

        // Use McCAMY formula
        // CCT = 449.0 × n3 + 3525.0 × n2 + 6823.3 × n + 5520.33
        // where n = (x - Xe) / (Ye - y)
        // Xe = 0.3320
        // Ye = 0.1858
        let color_n = (color_x - 0.3320) / (0.1858 - color_y);
        let cct = 449.0 * color_n.powi(3) + 3525.0 * color_n.powi(2) + 6823.3 * color_n + 5520.33;

        Some(cct)
    }

    pub fn ambient_light(&self, it: IntegrationTime) -> f32 {
        let g = self.green as f32;
        g * it.sensitivity()
    }
}

const DEVICE_ADDRESS: u8 = 0x10;

struct Register;

impl Register {
    const CONFIG: u8 = 0x00;
    const R_DATA: u8 = 0x08;
    const G_DATA: u8 = 0x09;
    const B_DATA: u8 = 0x0A;
    const W_DATA: u8 = 0x0B;
}

struct BitFlags;

impl BitFlags {
    const SHUTDOWN: u8 = 0b0000_0001;
    const AF: u8 = 0b0000_0010;
    const TRIG: u8 = 0b0000_0100;
}

/// VEML6040 device driver.
#[derive(Debug, Default)]
pub struct Veml6040<I2C> {
    /// The concrete I²C device implementation.
    pub(crate) i2c: I2C,
    /// Configuration register status.
    config: u8,
}

impl<I2C, E> Veml6040<I2C>
    where
        I2C: i2c::I2c + ErrorType<Error=E>,
{
    /// Create new instance of the VEML6040 device.
    pub fn new(i2c: I2C) -> Self {
        Veml6040 { i2c, config: 0 }
    }

    /// Destroy driver instance, return I²C bus instance.
    pub fn destroy(self) -> I2C {
        self.i2c
    }
}

impl<I2C, E> Veml6040<I2C>
    where
        I2C: i2c::I2c + ErrorType<Error=E>,
{
    /// Enable the sensor.
    pub async fn enable(&mut self) -> Result<(), Error<E>> {
        let config = self.config;
        self.write_config(config & !BitFlags::SHUTDOWN).await
    }

    /// Disable the sensor (shutdown).
    pub async fn disable(&mut self) -> Result<(), Error<E>> {
        let config = self.config;
        self.write_config(config | BitFlags::SHUTDOWN).await
    }

    /// Set the integration time.
    pub async fn set_integration_time(&mut self, it: IntegrationTime) -> Result<(), Error<E>> {
        const IT_BITS: u8 = 0b0111_0000;
        let config = self.config & !IT_BITS;
        match it {
            IntegrationTime::_40ms => self.write_config(config).await,
            IntegrationTime::_80ms => self.write_config(config | 0b0001_0000).await,
            IntegrationTime::_160ms => self.write_config(config | 0b0010_0000).await,
            IntegrationTime::_320ms => self.write_config(config | 0b0011_0000).await,
            IntegrationTime::_640ms => self.write_config(config | 0b0100_0000).await,
            IntegrationTime::_1280ms => self.write_config(config | 0b0101_0000).await,
        }
    }

    /// Set the measurement mode: `Auto`/`Manual`.
    pub async fn set_measurement_mode(&mut self, mode: MeasurementMode) -> Result<(), Error<E>> {
        let config = self.config;
        match mode {
            MeasurementMode::Auto => self.write_config(config & !BitFlags::AF).await,
            MeasurementMode::Manual => self.write_config(config | BitFlags::AF).await,
        }
    }

    /// Trigger a measurement when on `Manual` measurement mode.
    ///
    /// This is not necessary on `Auto` measurement mode.
    pub async fn trigger_measurement(&mut self) -> Result<(), Error<E>> {
        // This bit is not stored to avoid unintended triggers.
        self.i2c
            .write(DEVICE_ADDRESS, &[Register::CONFIG, self.config | BitFlags::TRIG, 0])
            .await
            .map_err(Error::I2C)
    }

    pub async fn write_config(&mut self, config: u8) -> Result<(), Error<E>> {
        self.i2c.write(DEVICE_ADDRESS, &[Register::CONFIG, config, 0]).await.map_err(Error::I2C)?;
        self.config = config;
        Ok(())
    }
}

impl<I2C, E> Veml6040<I2C>
    where
        I2C: i2c::I2c + ErrorType<Error=E>,
{
    pub async fn read_all_channels_with_oversampling(&mut self, it: IntegrationTime, oversample: usize) -> Result<AllChannelMeasurement, Error<E>> {
        self.set_integration_time(it).await?;
        let (mut r, mut g, mut b, mut w) = (0f32, 0f32, 0f32, 0f32);

        for index in 1..oversample + 1 {
            let index = index as f32;
            Timer::after(Duration::from_millis(it.get_duration_ms() + 2)).await;
            let measurement = self.read_all_channels_one_by_one().await?;

            r += (measurement.red as f32 - r) / index;
            g += (measurement.green as f32 - g) / index;
            b += (measurement.blue as f32 - b) / index;
            w += (measurement.white as f32 - w) / index;
        }

        Ok(AllChannelMeasurement {
            red: r as u16,
            green: g as u16,
            blue: b as u16,
            white: w as u16,
        })
    }

    pub async fn read_all_channels_one_by_one(&mut self) -> Result<AllChannelMeasurement, Error<E>> {
        let red = self.read_red_channel().await?;
        let green = self.read_green_channel().await?;
        let blue = self.read_blue_channel().await?;
        let white = self.read_white_channel().await?;

        Ok(AllChannelMeasurement {
            red,
            green,
            blue,
            white,
        })
    }

    /// Read the red channel measurement data.
    pub async fn read_red_channel(&mut self) -> Result<u16, Error<E>> {
        self.read_channel(Register::R_DATA).await
    }

    /// Read the green channel measurement data.
    pub async fn read_green_channel(&mut self) -> Result<u16, Error<E>> {
        self.read_channel(Register::G_DATA).await
    }

    /// Read the blue channel measurement data.
    pub async fn read_blue_channel(&mut self) -> Result<u16, Error<E>> {
        self.read_channel(Register::B_DATA).await
    }

    /// Read the white channel measurement data.
    pub async fn read_white_channel(&mut self) -> Result<u16, Error<E>> {
        self.read_channel(Register::W_DATA).await
    }

    /// Read the measurement data of all channels at once.
    pub async fn read_all_channels(&mut self) -> Result<AllChannelMeasurement, Error<E>> {
        let mut data = [0; 8];
        self.i2c
            .write_read(DEVICE_ADDRESS, &[Register::R_DATA], &mut data)
            .await
            .map_err(Error::I2C)?;

        Ok(Self::convert_buffer(&data))
    }

    fn convert_buffer(data: &[u8]) -> AllChannelMeasurement {
        AllChannelMeasurement {
            red: u16::from(data[1]) << 8 | u16::from(data[0]),
            green: u16::from(data[3]) << 8 | u16::from(data[2]),
            blue: u16::from(data[5]) << 8 | u16::from(data[4]),
            white: u16::from(data[7]) << 8 | u16::from(data[6]),
        }
    }

    pub async fn read_channel(&mut self, first_register: u8) -> Result<u16, Error<E>> {
        let mut data = [0; 2];
        self.i2c
            .write_read(DEVICE_ADDRESS, &[first_register], &mut data)
            .await
            .map_err(Error::I2C)
            .and(Ok(u16::from(data[1]) << 8 | u16::from(data[0])))
    }
}
