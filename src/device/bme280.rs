

use embassy_time::{Duration, Timer};
use thiserror_no_std::Error;

use crate::common::bitbang;
use crate::common::device::error::CustomI2CError;

pub(crate) const BME280_I2C_ADDR_PRIMARY: u8 = 0x76;
pub(crate) const BME280_I2C_ADDR_SECONDARY: u8 = 0x77;

pub(crate) const BME280_PWR_CTRL_ADDR: u8 = 0xF4;
pub(crate) const BME280_CTRL_HUM_ADDR: u8 = 0xF2;
pub(crate) const BME280_CTRL_MEAS_ADDR: u8 = 0xF4;
pub(crate) const BME280_CONFIG_ADDR: u8 = 0xF5;

pub(crate) const BME280_RESET_ADDR: u8 = 0xE0;
pub(crate) const BME280_SOFT_RESET_CMD: u8 = 0xB6;

pub(crate) const BME280_CHIP_ID: u8 = 0x60;
pub(crate) const BMP280_CHIP_ID: u8 = 0x58;
pub(crate) const BME280_CHIP_ID_ADDR: u8 = 0xD0;

pub(crate) const BME280_DATA_ADDR: u8 = 0xF7;
pub(crate) const BME280_P_T_H_DATA_LEN: usize = 8;

pub(crate) const BME280_P_T_CALIB_DATA_ADDR: u8 = 0x88;
pub(crate) const BME280_P_T_CALIB_DATA_LEN: usize = 26;

pub(crate) const BME280_H_CALIB_DATA_ADDR: u8 = 0xE1;
pub(crate) const BME280_H_CALIB_DATA_LEN: usize = 7;

pub(crate) const BME280_TEMP_MIN: f32 = -40.0;
pub(crate) const BME280_TEMP_MAX: f32 = 85.0;

pub(crate) const BME280_PRESSURE_MIN: f32 = 30000.0;
pub(crate) const BME280_PRESSURE_MAX: f32 = 110000.0;

pub(crate) const BME280_HUMIDITY_MIN: f32 = 0.0;
pub(crate) const BME280_HUMIDITY_MAX: f32 = 100.0;

pub(crate) const BME280_SLEEP_MODE: u8 = 0x00;
pub(crate) const BME280_FORCED_MODE: u8 = 0x01;
pub(crate) const BME280_NORMAL_MODE: u8 = 0x03;

pub(crate) const BME280_SENSOR_MODE_MSK: u8 = 0x03;

pub(crate) const BME280_CTRL_HUM_MSK: u8 = 0x07;

pub(crate) const BME280_CTRL_PRESS_MSK: u8 = 0x1C;
pub(crate) const BME280_CTRL_PRESS_POS: u8 = 0x02;

pub(crate) const BME280_CTRL_TEMP_MSK: u8 = 0xE0;
pub(crate) const BME280_CTRL_TEMP_POS: u8 = 0x05;

pub(crate) const BME280_FILTER_MSK: u8 = 0x1C;
pub(crate) const BME280_FILTER_POS: u8 = 0x02;
pub(crate) const BME280_FILTER_COEFF_OFF: u8 = 0x00;
pub(crate) const BME280_FILTER_COEFF_2: u8 = 0x01;
pub(crate) const BME280_FILTER_COEFF_4: u8 = 0x02;
pub(crate) const BME280_FILTER_COEFF_8: u8 = 0x03;
pub(crate) const BME280_FILTER_COEFF_16: u8 = 0x04;

pub(crate) const BME280_OVERSAMPLING_1X: u8 = 0x01;
pub(crate) const BME280_OVERSAMPLING_2X: u8 = 0x02;
pub(crate) const BME280_OVERSAMPLING_4X: u8 = 0x03;
pub(crate) const BME280_OVERSAMPLING_8X: u8 = 0x04;
pub(crate) const BME280_OVERSAMPLING_16X: u8 = 0x05;

macro_rules! concat_bytes {
    ($msb:expr, $lsb:expr) => {
        (($msb as u16) << 8) | ($lsb as u16)
    };
}

macro_rules! set_bits {
    ($reg_data:expr, $mask:expr, $pos:expr, $data:expr) => {
        ($reg_data & !$mask) | (($data << $pos) & $mask)
    };
}



pub(crate) struct Bme280<'a, I: embedded_hal_async::i2c::I2c> {
    address: u8,
    interface: &'a mut I,
    calibration: Option<CalibrationData>,
}

impl<'a,  I: embedded_hal_async::i2c::I2c> Bme280<'a,  I>  where Bme280Error: core::convert::From<<I as embedded_hal_async::i2c::ErrorType>::Error> {
    pub fn new(interface: &'a mut I, address: u8) -> Self {
        Self {
            address,
            interface,
            calibration: None,
        }
    }

    pub fn new_primary(interface: &'a mut I) -> Self {
        Self::new(interface, BME280_I2C_ADDR_PRIMARY)
    }

    pub fn new_secondary(interface: &'a mut I) -> Self {
        Self::new(interface, BME280_I2C_ADDR_SECONDARY)
    }

    pub async fn init(
        &mut self,
        config: Configuration,
    ) -> Result<(), Bme280Error> {
        self.verify_chip_id().await?;
        self.soft_reset().await?;
        self.calibrate().await?;
        self.configure(config).await
    }

    async fn verify_chip_id(&mut self) -> Result<(), Bme280Error> {
        let chip_id = self.read_register(BME280_CHIP_ID_ADDR).await?;
        if chip_id == BME280_CHIP_ID || chip_id == BMP280_CHIP_ID {
            Ok(())
        } else {
            Err(Bme280Error::UnsupportedChip(chip_id))
        }
    }

    pub async fn write_register(&mut self, register: u8, payload: u8) -> Result<(), Bme280Error> {
        self.interface.write(self.address, &[register, payload]).await?;
        Ok(())
    }

    pub async fn read_register(&mut self, register: u8) -> Result<u8, Bme280Error> {
        let mut buf = [0u8; 1];
        self.interface.write_read(self.address, &[register], &mut buf).await?;
        Ok(buf[0])
    }

    pub async fn read_chip_id(&mut self) -> Result<u8, Bme280Error> {
        self.read_register(BME280_CHIP_ID_ADDR).await
    }

    pub async fn soft_reset(&mut self) -> Result<(), Bme280Error> {
        self.write_register(BME280_RESET_ADDR, BME280_SOFT_RESET_CMD).await?;
        Timer::after(Duration::from_millis(2)).await;  // startup 2ms
        Ok(())
    }

    async fn calibrate(&mut self) -> Result<(), Bme280Error> {
        let pt_calib_data = self
            .read_pt_calib_data(BME280_P_T_CALIB_DATA_ADDR)
            .await?;
        let h_calib_data = self
            .read_h_calib_data(BME280_H_CALIB_DATA_ADDR)
            .await?;
        self.calibration = Some(parse_calib_data(&pt_calib_data, &h_calib_data));
        Ok(())
    }

    pub async fn read_data(
        &mut self,
        register: u8,
    ) -> Result<[u8; BME280_P_T_H_DATA_LEN], Bme280Error> {
        let mut data = [0; BME280_P_T_H_DATA_LEN];
        self.interface
            .write_read(self.address, &[register], &mut data).await?;
        Ok(data)
    }

    pub async fn read_pt_calib_data(&mut self, register: u8) -> Result<[u8; BME280_P_T_CALIB_DATA_LEN], Bme280Error> {
        let mut data = [0; BME280_P_T_CALIB_DATA_LEN];
        self.interface
            .write_read(self.address, &[register], &mut data)
            .await?;
        Ok(data)
    }

    pub async fn read_h_calib_data(&mut self, register: u8) -> Result<[u8; BME280_H_CALIB_DATA_LEN], Bme280Error> {
        let mut data = [0; BME280_H_CALIB_DATA_LEN];
        self.interface
            .write_read(self.address, &[register], &mut data)
            .await?;
        Ok(data)
    }

    pub async fn configure(
        &mut self,
        config: Configuration,
    ) -> Result<(), Bme280Error> {
        match self.mode().await? {
            SensorMode::Sleep => {}
            _ => self.soft_reset().await?,
        };

        self.write_register(
            BME280_CTRL_HUM_ADDR,
            config.humidity_oversampling.bits() & BME280_CTRL_HUM_MSK,
        )
            .await?;

        // As per the datasheet, the ctrl_meas register needs to be written after
        // the ctrl_hum register for changes to take effect.
        let data = self.read_register(BME280_CTRL_MEAS_ADDR).await?;
        let data = set_bits!(
            data,
            BME280_CTRL_PRESS_MSK,
            BME280_CTRL_PRESS_POS,
            config.pressure_oversampling.bits()
        );
        let data = set_bits!(
            data,
            BME280_CTRL_TEMP_MSK,
            BME280_CTRL_TEMP_POS,
            config.temperature_oversampling.bits()
        );
        self.write_register(BME280_CTRL_MEAS_ADDR, data).await?;

        let data = self.read_register(BME280_CONFIG_ADDR).await?;
        let data = set_bits!(
            data,
            BME280_FILTER_MSK,
            BME280_FILTER_POS,
            config.iir_filter.bits()
        );
        self.write_register(BME280_CONFIG_ADDR, data)
            .await
    }

    pub async fn mode(&mut self) -> Result<SensorMode, Bme280Error> {
        let data = self.read_register(BME280_PWR_CTRL_ADDR).await?;
        match data & BME280_SENSOR_MODE_MSK {
            BME280_SLEEP_MODE => Ok(SensorMode::Sleep),
            BME280_FORCED_MODE => Ok(SensorMode::Forced),
            BME280_NORMAL_MODE => Ok(SensorMode::Normal),
            _ => Err(Bme280Error::InvalidData),
        }
    }

    pub async fn forced(&mut self) -> Result<(), Bme280Error> {
        self.set_mode(BME280_FORCED_MODE).await
    }

    pub async fn set_mode(
        &mut self,
        mode: u8,
    ) -> Result<(), Bme280Error> {
        match self.mode().await? {
            SensorMode::Sleep => {}
            _ => self.soft_reset().await?,
        };
        let data = self.read_register(BME280_PWR_CTRL_ADDR).await?;
        let data = set_bits!(data, BME280_SENSOR_MODE_MSK, 0, mode);
        self.write_register(BME280_PWR_CTRL_ADDR, data)
            .await
    }

    /// Captures and processes sensor data for temperature, pressure, and humidity
    pub async fn measure(
        &mut self,
    ) -> Result<Measurements, Bme280Error> {
        self.forced().await?;
        Timer::after(Duration::from_millis(40)).await;
        let measurements = self.read_data(BME280_DATA_ADDR).await?;
        match self.calibration.as_mut() {
            Some(calibration) => {
                let measurements = Measurements::parse(measurements, &mut *calibration)?;
                Ok(measurements)
            }
            None => Err(Bme280Error::NoCalibrationData),
        }
    }
}


fn parse_calib_data(
    pt_data: &[u8; BME280_P_T_CALIB_DATA_LEN],
    h_data: &[u8; BME280_H_CALIB_DATA_LEN],
) -> CalibrationData {
    let dig_t1 = concat_bytes!(pt_data[1], pt_data[0]);
    let dig_t2 = concat_bytes!(pt_data[3], pt_data[2]) as i16;
    let dig_t3 = concat_bytes!(pt_data[5], pt_data[4]) as i16;
    let dig_p1 = concat_bytes!(pt_data[7], pt_data[6]);
    let dig_p2 = concat_bytes!(pt_data[9], pt_data[8]) as i16;
    let dig_p3 = concat_bytes!(pt_data[11], pt_data[10]) as i16;
    let dig_p4 = concat_bytes!(pt_data[13], pt_data[12]) as i16;
    let dig_p5 = concat_bytes!(pt_data[15], pt_data[14]) as i16;
    let dig_p6 = concat_bytes!(pt_data[17], pt_data[16]) as i16;
    let dig_p7 = concat_bytes!(pt_data[19], pt_data[18]) as i16;
    let dig_p8 = concat_bytes!(pt_data[21], pt_data[20]) as i16;
    let dig_p9 = concat_bytes!(pt_data[23], pt_data[22]) as i16;
    let dig_h1 = pt_data[25];
    let dig_h2 = concat_bytes!(h_data[1], h_data[0]) as i16;
    let dig_h3 = h_data[2];
    let dig_h4 = (h_data[3] as i8 as i16 * 16) | ((h_data[4] as i8 as i16) & 0x0F);
    let dig_h5 = (h_data[5] as i8 as i16 * 16) | (((h_data[4] as i8 as i16) & 0xF0) >> 4);
    let dig_h6 = h_data[6] as i8;

    CalibrationData {
        dig_t1,
        dig_t2,
        dig_t3,
        dig_p1,
        dig_p2,
        dig_p3,
        dig_p4,
        dig_p5,
        dig_p6,
        dig_p7,
        dig_p8,
        dig_p9,
        dig_h1,
        dig_h2,
        dig_h3,
        dig_h4,
        dig_h5,
        dig_h6,
        t_fine: 0,
    }
}


struct CalibrationData {
    dig_t1: u16,
    dig_t2: i16,
    dig_t3: i16,
    dig_p1: u16,
    dig_p2: i16,
    dig_p3: i16,
    dig_p4: i16,
    dig_p5: i16,
    dig_p6: i16,
    dig_p7: i16,
    dig_p8: i16,
    dig_p9: i16,
    dig_h1: u8,
    dig_h2: i16,
    dig_h3: u8,
    dig_h4: i16,
    dig_h5: i16,
    dig_h6: i8,
    t_fine: i32,
}


#[derive(Debug, Default, Copy, Clone)]
pub struct Configuration {
    temperature_oversampling: Oversampling,
    pressure_oversampling: Oversampling,
    humidity_oversampling: Oversampling,
    iir_filter: IIRFilter,
}

impl Configuration {
    /// Sets the temperature oversampling setting.
    pub fn with_temperature_oversampling(mut self, oversampling: Oversampling) -> Self {
        self.temperature_oversampling = oversampling;
        self
    }

    /// Sets the pressure oversampling setting.
    pub fn with_pressure_oversampling(mut self, oversampling: Oversampling) -> Self {
        self.pressure_oversampling = oversampling;
        self
    }

    /// Sets the humidity oversampling setting
    pub fn with_humidity_oversampling(mut self, oversampling: Oversampling) -> Self {
        self.humidity_oversampling = oversampling;
        self
    }

    /// Sets the IIR filter setting.
    pub fn with_iir_filter(mut self, filter: IIRFilter) -> Self {
        self.iir_filter = filter;
        self
    }
}

#[derive(Debug)]
pub struct Measurements {
    /// temperature in degrees celsius
    pub temperature: f32,
    /// pressure in pascals
    pub pressure: f32,
    /// percent relative humidity (`0` with BMP280)
    pub humidity: f32,
}


impl Measurements {
    fn parse(
        data: [u8; BME280_P_T_H_DATA_LEN],
        calibration: &mut CalibrationData,
    ) -> Result<Self, Bme280Error> {
        let data_msb = (data[0] as u32) << 12;
        let data_lsb = (data[1] as u32) << 4;
        let data_xlsb = (data[2] as u32) >> 4;
        let pressure = data_msb | data_lsb | data_xlsb;

        let data_msb = (data[3] as u32) << 12;
        let data_lsb = (data[4] as u32) << 4;
        let data_xlsb = (data[5] as u32) >> 4;
        let temperature = data_msb | data_lsb | data_xlsb;

        let data_msb = (data[6] as u32) << 8;
        let data_lsb = data[7] as u32;
        let humidity = data_msb | data_lsb;

        let temperature = Measurements::compensate_temperature(temperature, calibration)?;
        let pressure = Measurements::compensate_pressure(pressure, calibration)?;
        let humidity = Measurements::compensate_humidity(humidity, calibration)?;

        Ok(Measurements {
            temperature,
            pressure,
            humidity,
        })
    }

    fn compensate_temperature(
        uncompensated: u32,
        calibration: &mut CalibrationData,
    ) -> Result<f32, Bme280Error> {
        let var1 = uncompensated as f32 / 16384.0 - calibration.dig_t1 as f32 / 1024.0;
        let var1 = var1 * calibration.dig_t2 as f32;
        let var2 = uncompensated as f32 / 131072.0 - calibration.dig_t1 as f32 / 8192.0;
        let var2 = var2 * var2 * calibration.dig_t3 as f32;

        calibration.t_fine = (var1 + var2) as i32;

        let temperature = (var1 + var2) / 5120.0;
        let temperature = if temperature < BME280_TEMP_MIN {
            BME280_TEMP_MIN
        } else if temperature > BME280_TEMP_MAX {
            BME280_TEMP_MAX
        } else {
            temperature
        };
        Ok(temperature)
    }

    fn compensate_pressure(
        uncompensated: u32,
        calibration: &mut CalibrationData,
    ) -> Result<f32, Bme280Error> {
        let var1 = calibration.t_fine as f32 / 2.0 - 64000.0;
        let var2 = var1 * var1 * calibration.dig_p6 as f32 / 32768.0;
        let var2 = var2 + var1 * calibration.dig_p5 as f32 * 2.0;
        let var2 = var2 / 4.0 + calibration.dig_p4 as f32 * 65536.0;
        let var3 = calibration.dig_p3 as f32 * var1 * var1 / 524288.0;
        let var1 = (var3 + calibration.dig_p2 as f32 * var1) / 524288.0;
        let var1 = (1.0 + var1 / 32768.0) * calibration.dig_p1 as f32;

        let pressure = if var1 > 0.0 {
            let pressure = 1048576.0 - uncompensated as f32;
            let pressure = (pressure - (var2 / 4096.0)) * 6250.0 / var1;
            let var1 = calibration.dig_p9 as f32 * pressure * pressure / 2147483648.0;
            let var2 = pressure * calibration.dig_p8 as f32 / 32768.0;
            let pressure = pressure + (var1 + var2 + calibration.dig_p7 as f32) / 16.0;
            if pressure < BME280_PRESSURE_MIN {
                BME280_PRESSURE_MIN
            } else if pressure > BME280_PRESSURE_MAX {
                BME280_PRESSURE_MAX
            } else {
                pressure
            }
        } else {
            return Err(Bme280Error::InvalidData);
        };
        Ok(pressure)
    }

    fn compensate_humidity(
        uncompensated: u32,
        calibration: &mut CalibrationData,
    ) -> Result<f32, Bme280Error> {
        let var1 = calibration.t_fine as f32 - 76800.0;
        let var2 = calibration.dig_h4 as f32 * 64.0 + (calibration.dig_h5 as f32 / 16384.0) * var1;
        let var3 = uncompensated as f32 - var2;
        let var4 = calibration.dig_h2 as f32 / 65536.0;
        let var5 = 1.0 + (calibration.dig_h3 as f32 / 67108864.0) * var1;
        let var6 = 1.0 + (calibration.dig_h6 as f32 / 67108864.0) * var1 * var5;
        let var6 = var3 * var4 * (var5 * var6);

        let humidity = var6 * (1.0 - calibration.dig_h1 as f32 * var6 / 524288.0);
        let humidity = if humidity < BME280_HUMIDITY_MIN {
            BME280_HUMIDITY_MIN
        } else if humidity > BME280_HUMIDITY_MAX {
            BME280_HUMIDITY_MAX
        } else {
            humidity
        };
        Ok(humidity)
    }
}


/// Oversampling settings for temperature, pressure, and humidity measurements.
/// See sections 3.4ff of the manual for measurement flow and recommended values.
/// The default is 1x, i.e., no oversampling.
#[derive(Debug, Copy, Clone)]
pub enum Oversampling {
    /// Disables oversampling.
    /// Without IIR filtering, this sets the resolution of temperature and pressure measurements
    /// to 16 bits.
    Oversampling1X,
    /// Configures 2x oversampling.
    /// This increases the resolution of temperature and pressure measurements to 17 bits without
    /// IIR filtering.
    Oversampling2X,
    /// Configures 4x oversampling.
    /// This increases the resolution of temperature and pressure measurements to 18 bits without
    /// IIR filtering.
    Oversampling4X,
    /// Configures 8x oversampling.
    /// This increases the resolution of temperature and pressure measurements to 19 bits without
    /// IIR filtering.
    Oversampling8X,
    /// Configures 16x oversampling.
    /// This increases the resolution of temperature and pressure measurements to 20 bits,
    /// regardless of IIR filtering.
    Oversampling16X,
}


impl Oversampling {
    fn bits(&self) -> u8 {
        match self {
            Oversampling::Oversampling1X => BME280_OVERSAMPLING_1X,
            Oversampling::Oversampling2X => BME280_OVERSAMPLING_2X,
            Oversampling::Oversampling4X => BME280_OVERSAMPLING_4X,
            Oversampling::Oversampling8X => BME280_OVERSAMPLING_8X,
            Oversampling::Oversampling16X => BME280_OVERSAMPLING_16X,
        }
    }
}

impl Default for Oversampling {
    fn default() -> Self {
        Self::Oversampling1X
    }
}

/// Lowpass filter settings for pressure and temperature values.
/// See section 3.4.4 of the datasheet for more information on this.
/// The default setting is disabled.
#[derive(Debug, Copy, Clone)]
#[derive(Default)]
pub enum IIRFilter {
    /// Disables the IIR filter.
    /// The resolution of pressure and temperature measurements is dictated by their respective
    /// oversampling settings.
    #[default]
    Off,

    /// Sets the IIR filter coefficient to 2.
    /// This increases the resolution of the pressure and temperature measurements to 20 bits.
    /// See sections 3.4.4 and 3.5 of the datasheet for more information.
    Coefficient2,

    /// Sets the IIR filter coefficient to 4.
    Coefficient4,

    /// Sets the IIR filter coefficient to 8.
    Coefficient8,

    /// Sets the IIR filter coefficient to 16.
    Coefficient16,
}

impl IIRFilter {
    fn bits(&self) -> u8 {
        match self {
            IIRFilter::Off => BME280_FILTER_COEFF_OFF,
            IIRFilter::Coefficient2 => BME280_FILTER_COEFF_2,
            IIRFilter::Coefficient4 => BME280_FILTER_COEFF_4,
            IIRFilter::Coefficient8 => BME280_FILTER_COEFF_8,
            IIRFilter::Coefficient16 => BME280_FILTER_COEFF_16,
        }
    }
}



#[derive(Debug)]
pub enum SensorMode {
    Sleep,
    Forced,
    Normal,
}

/// BME280 errors
#[derive(Error, Debug)]
pub enum Bme280Error {
    #[error("Failed to compensate a raw measurement")]
    CompensationFailed,

    #[error("I²C error")]
    BitbangBus(#[from] bitbang::i2c::BitbangI2CError),

    #[error("I²C error")]
    NativeBus(#[from] CustomI2CError),

    #[error("Failed to parse sensor data")]
    InvalidData,

    #[error("No calibration data is available (probably forgot to call or check BME280::init for failure)")]
    NoCalibrationData,

    #[error("Chip ID doesn't match expected value")]
    UnsupportedChip(u8),

    #[error("Delay error")]
    Delay,
}
