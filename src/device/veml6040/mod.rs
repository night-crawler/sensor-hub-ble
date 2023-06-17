use defmt::info;
use embedded_hal_async::i2c;
use embedded_hal_async::i2c::ErrorType;

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
    i2c: I2C,
    /// Configuration register status.
    config: u8,
}

impl<I2C, E> Veml6040<I2C>
where
    I2C: i2c::I2c + ErrorType<Error = E>,
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
    I2C: i2c::I2c + ErrorType<Error = E>,
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
            .write(
                DEVICE_ADDRESS,
                &[Register::CONFIG, self.config | BitFlags::TRIG, 0],
            )
            .await
            .map_err(Error::I2C)
    }

    pub async fn write_config(&mut self, config: u8) -> Result<(), Error<E>> {
        self.i2c
            .write(DEVICE_ADDRESS, &[Register::CONFIG, config, 0])
            .await
            .map_err(Error::I2C)?;
        self.config = config;
        Ok(())
    }
}

impl<I2C, E> Veml6040<I2C>
where
    I2C: i2c::I2c + ErrorType<Error = E>,
{
    /// Read the red channel measurement data.
    pub async fn read_red_channel(&mut self) -> Result<u16, Error<E>> {
        self.read_channel(Register::R_DATA).await
    }

    /// Read the green channel measurement data.
    pub async fn read_green_channel(&mut self) -> Result<u16, Error<E>> {
        self.read_channel(u8::MAX).await
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
        Ok(AllChannelMeasurement {
            red: u16::from(data[1]) << 8 | u16::from(data[0]),
            green: u16::from(data[3]) << 8 | u16::from(data[2]),
            blue: u16::from(data[5]) << 8 | u16::from(data[4]),
            white: u16::from(data[7]) << 8 | u16::from(data[6]),
        })
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
