use embassy_nrf::peripherals::TWISPI0;
use embassy_nrf::twim;
use embassy_nrf::twim::{Config, Twim};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use nrf_softdevice::ble::Connection;

use crate::ble_debug;
use crate::common::ble::services::BleServer;
use crate::common::device::bme280;
use crate::common::device::device_manager::{I2CPins, Irqs};
use crate::common::device::error::CustomI2CError;

pub(crate) async fn read_i2c0<'a>(i2c_pins: &mut I2CPins<TWISPI0>, server: &'a BleServer, connection: &'a Connection) {
    let mut config = Config::default();
    config.frequency = i2c_pins.config.frequency;
    config.sda_high_drive = i2c_pins.config.sda_high_drive;
    config.sda_pullup = i2c_pins.config.sda_pullup;
    config.scl_high_drive = i2c_pins.config.scl_high_drive;
    config.scl_pullup = i2c_pins.config.scl_pullup;

    let mut twim: Twim<TWISPI0> = Twim::new(&mut i2c_pins.twim, Irqs, &mut i2c_pins.sda, &mut i2c_pins.scl, config);

    let a: Mutex<ThreadModeRawMutex, Twim<TWISPI0>> = Mutex::new(twim);

    let mut bme = bme280::Bme280::new_primary(&a);
    let bme_config = bme280::Configuration::default()
        .with_humidity_oversampling(bme280::Oversampling::Oversampling16X)
        .with_temperature_oversampling(bme280::Oversampling::Oversampling16X)
        .with_pressure_oversampling(bme280::Oversampling::Oversampling16X)
        .with_iir_filter(bme280::IIRFilter::Coefficient16);


    let mut bme_error = bme.init(bme_config).await.is_err();
    loop {
        if bme_error {
            bme_error = match bme.soft_reset().await {
                Ok(_) => {
                    ble_debug!("Reset OK");
                    false
                },
                Err(err) => {
                    ble_debug!("Failed to reset: {}", err);
                    true
                }
            };
            ble_debug!("Error status: {}", bme_error);
            if !bme_error {
                ble_debug!("BME 280 Init result: {:x?}", bme.init(bme_config).await);
            }
        }

        if !bme_error {
            bme_error = match bme.measure().await {
                Ok(measurements) => {
                    ble_debug!("t: {}, h: {}, p: {}", measurements.temperature, measurements.humidity, measurements.pressure);
                    false
                }
                Err(err) => {
                    ble_debug!("E");
                    Timer::after(Duration::from_millis(50)).await;
                    ble_debug!("E {:?}", err);
                    true
                }
            };
        }

        Timer::after(Duration::from_millis(1000)).await;
        ble_debug!("Here");
    }
}


pub trait I2CWrapper {
    async fn write_read(&mut self, address: u8, wr_buffer: &[u8], rd_buffer: &mut [u8]) -> Result<(), CustomI2CError>;
    async fn write(&mut self, address: u8, buffer: &[u8]) -> Result<(), CustomI2CError>;
    async fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), CustomI2CError>;
}

impl<'a, T: twim::Instance> I2CWrapper for Twim<'a, T> {
    async fn write_read(&mut self, address: u8, wr_buffer: &[u8], rd_buffer: &mut [u8]) -> Result<(), CustomI2CError> {
        // https://docs.embassy.dev/embassy-nrf/git/nrf52840/index.html#easydma-considerations
        self.write_read(address, wr_buffer, rd_buffer).await?;
        Ok(())
    }

    async fn write(&mut self, address: u8, buffer: &[u8]) -> Result<(), CustomI2CError> {
        self.write(address, buffer).await?;
        Ok(())
    }

    async fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), CustomI2CError> {
        self.read(address, buffer).await?;
        Ok(())
    }
}

