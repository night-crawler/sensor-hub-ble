use core::ops::DerefMut;

use cortex_m::prelude::_embedded_hal_blocking_spi_Write;
use defmt::info;
use embassy_nrf::gpio::{Flex, Level, Output, OutputDrive, Pull};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use futures::{select_biased, FutureExt};
use nrf_softdevice::ble::Connection;
use rclite::Arc;

use crate::ble_debug;
use crate::ble_notify;
use crate::common::bitbang;
use crate::common::ble::conv::ConvExt;
use crate::common::ble::services::BleServer;
use crate::common::ble::{BME_TASK_CONDITION, SERVER};
use crate::common::device::bme280::{Bme280Error, BME280_SLEEP_MODE};
use crate::common::device::device_manager::BitbangI2CPins;
use crate::common::device::lis2h12::reg::{FifoMode, FullScale, Odr};
use crate::common::device::lis2h12::{Lis2dh12, SlaveAddr};
use crate::common::device::veml6040::Veml6040;
use crate::common::device::{bme280, veml6040};

#[embassy_executor::task]
pub(crate) async fn read_i2c0_task(i2c_pins: Arc<Mutex<ThreadModeRawMutex, BitbangI2CPins>>) {
    loop {
        let bme_fut = read_bme_task(Arc::clone(&i2c_pins), SERVER.get());
        let accel_fut = read_accel_task(Arc::clone(&i2c_pins), SERVER.get());
        let color_fut = read_veml_task(Arc::clone(&i2c_pins), SERVER.get());

        select_biased! {
             result = bme_fut.fuse() => {
                if let Err(err) = result {
                    info!("BME Error");
                    ble_debug!("Err: {}", err);
                }
            },
            result = accel_fut.fuse() => {
                if let Err(err) = result {
                    info!("Accel Error");
                    ble_debug!("Err: {:?}", err);
                }
            },
            result = color_fut.fuse() => {
                if let Err(err) = result {
                    info!("Color Error");
                    ble_debug!("Err: {:?}", err);
                }
            },
        }

        Timer::after(Duration::from_millis(1000)).await;
    }
}

async fn read_bme_task(
    i2c_pins: Arc<Mutex<ThreadModeRawMutex, BitbangI2CPins>>,
    server: &BleServer,
) -> Result<(), Bme280Error> {
    loop {
        let _token = BME_TASK_CONDITION.lock().await;
        let measurements = {
            let mut i2c_pins = i2c_pins.lock().await;
            let i2c_pins = i2c_pins.deref_mut();
            let mut sda = Flex::new(&mut i2c_pins.sda);
            sda.set_as_input_output(Pull::None, OutputDrive::Standard);
            let mut i2c = bitbang::i2c::BitbangI2C::new(
                Output::new(&mut i2c_pins.scl, Level::High, OutputDrive::Standard),
                sda,
                Default::default(),
            );

            let mut bme = bme280::Bme280::new_primary(&mut i2c);
            let bme_config = bme280::Configuration::default()
                .with_humidity_oversampling(bme280::Oversampling::Oversampling8X)
                .with_temperature_oversampling(bme280::Oversampling::Oversampling8X)
                .with_pressure_oversampling(bme280::Oversampling::Oversampling8X)
                .with_iir_filter(bme280::IIRFilter::Coefficient8);

            info!("Prepared BME280 config");

            bme.init(bme_config).await?;
            Timer::after(Duration::from_millis(10)).await;
            let measurements = bme.measure().await?;
            info!(
                "t: {}, h: {}, p: {}",
                measurements.temperature, measurements.humidity, measurements.pressure
            );

            Timer::after(Duration::from_millis(100)).await;

            let measurements = bme.measure().await?;

            bme.set_mode(BME280_SLEEP_MODE).await?;

            measurements
        };

        info!(
            "Final measurements: t: {}, h: {}, p: {}",
            measurements.temperature, measurements.humidity, measurements.pressure
        );

        let temperature = measurements.temperature.as_temp();
        let humidity = measurements.humidity.as_humidity();
        let pressure = measurements.pressure.as_pressure();

        for connection in Connection::iter() {
            ble_notify!(server.bme280, &connection, temperature, &temperature);
            ble_notify!(server.bme280, &connection, humidity, &humidity);
            ble_notify!(server.bme280, &connection, pressure, &pressure);
        }

        Timer::after(Duration::from_millis(1000)).await;
    }
}

async fn read_accel_task(
    i2c_pins: Arc<Mutex<ThreadModeRawMutex, BitbangI2CPins>>,
    _server: &BleServer,
) -> Result<(), accelerometer::Error<bitbang::i2c::BitbangI2CError>> {
    loop {
        Timer::after(Duration::from_millis(1000000)).await;
        {
            let mut i2c_pins = i2c_pins.lock().await;

            let i2c_pins = i2c_pins.deref_mut();
            let mut sda = Flex::new(&mut i2c_pins.sda);
            sda.set_as_input_output(Pull::None, OutputDrive::Standard);
            let i2c = bitbang::i2c::BitbangI2C::new(
                Output::new(&mut i2c_pins.scl, Level::High, OutputDrive::Standard),
                sda,
                Default::default(),
            );

            let mut lis = Lis2dh12::new(i2c, SlaveAddr::Default).await?;
            info!("Initialized lis");
            lis.reset().await?;

            lis.set_odr(Odr::Hz50).await?;
            info!("Set ODR");

            lis.set_bdu(true).await?;
            info!("Set BDU");

            lis.set_fs(FullScale::G2).await?;

            lis.set_mode(crate::common::device::lis2h12::reg::Mode::Normal).await?;
            info!("Set mode");

            lis.enable_axis((true, true, true)).await?;
            info!("Enabled all axis");

            lis.enable_temp(true).await?;
            info!("Enabled temp");

            lis.enable_fifo(true).await?;
            info!("Fifo enabled");

            lis.set_fm(FifoMode::Bypass).await?;

            info!("status: {:?}", lis.get_status().await?);

            info!("temp status: {}", lis.get_temp_status().await?);

            info!("Sample rate: {}", lis.sample_rate().await?);

            for _ in 0..10 {
                let a = lis.accel_norm().await?;
                // info!("Accel: ({}, {}, {})", a.x, a.y, a.z);
                // info!("Temp out: {}", lis.get_temp_outf().await?);
                // info!("Stored samples: {}", lis.get_stored_samples().await?);
            }
        }

        Timer::after(Duration::from_millis(100000)).await;
    }
}

async fn read_veml_task(
    i2c_pins: Arc<Mutex<ThreadModeRawMutex, BitbangI2CPins>>,
    _server: &BleServer,
) -> Result<(), veml6040::Error<bitbang::i2c::BitbangI2CError>> {
    loop {
        Timer::after(Duration::from_millis(1000000)).await;

        let measurements = {
            let mut i2c_pins = i2c_pins.lock().await;

            let i2c_pins = i2c_pins.deref_mut();
            let mut sda = Flex::new(&mut i2c_pins.sda);
            sda.set_as_input_output(Pull::None, OutputDrive::Standard0Disconnect1);
            let i2c = bitbang::i2c::BitbangI2C::new(
                Output::new(&mut i2c_pins.scl, Level::High, OutputDrive::Standard0Disconnect1),
                sda,
                Default::default(),
            );

            let mut veml = Veml6040::new(i2c);
            info!("Initialized veml");

            // veml.enable().await?;
            // veml.set_measurement_mode(MeasurementMode::Auto).await?;
            // veml.set_integration_time(IntegrationTime::_40ms).await?;
            // does not work :(
            veml.write_config(
                0x00 + 0x00 + 0x00, // VEML6040_IT_40MS + VEML6040_AF_AUTO + VEML6040_SD_ENABLE
            )
            .await?;

            Timer::after(Duration::from_millis(40)).await;

            veml.read_all_channels().await?
        };
        info!("Final measurements: {}", measurements);

        Timer::after(Duration::from_millis(1000)).await;
    }
}

#[macro_export]
macro_rules! ble_notify {
    ($service:expr, $conn:expr, $characteristic:ident, $value:expr) => {
        paste::paste! {
            if let Err(err) = $service.[<$characteristic _notify>]($conn, $value) {
                $crate::ble_debug!("{} notify error: {:?} - {:?}", stringify!($characteristic), err, $value);
                if let Err(err) = $service.[<$characteristic _set>]($value) {
                    $crate::ble_debug!("{} notify error: {:?} - {:?}", stringify!($characteristic), err, $value);
                }
            }
        }
    };
}
