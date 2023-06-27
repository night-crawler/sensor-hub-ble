use core::ops::DerefMut;

use defmt::info;
use embassy_nrf::gpio::{Flex, Level, Output, OutputDrive, Pull};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use futures::{select_biased, FutureExt};
use nrf_softdevice::ble::Connection;
use rclite::Arc;

use crate::ble_debug;
use crate::common::bitbang;
use crate::common::ble::conv::ConvExt;
use crate::common::ble::services::BleServer;
use crate::common::ble::{
    ACCELEROMETER_EVENT_PROCESSOR, BME_EVENT_PROCESSOR, COLOR_EVENT_PROCESSOR, SERVER,
};
use crate::common::device::bme280::{Bme280Error, BME280_SLEEP_MODE};
use crate::common::device::device_manager::BitbangI2CPins;
use crate::common::device::lis2dh12::reg::{FifoMode, FullScale, Odr};
use crate::common::device::lis2dh12::{Lis2dh12, SlaveAddr};
use crate::common::device::veml6040::Veml6040;
use crate::common::device::{bme280, veml6040};
use crate::notify_all;

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
                    ble_debug!("BME error: {}", err);
                }
            },
            result = accel_fut.fuse() => {
                if let Err(err) = result {
                    info!("Accel Error");
                    ble_debug!("Accel error: {:?}", err);
                }
            },
            result = color_fut.fuse() => {
                if let Err(err) = result {
                    info!("Color Error");
                    ble_debug!("Color error: {:?}", err);
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
        let _token = BME_EVENT_PROCESSOR.wait_for_condition().await;

        let measurements = {
            let mut i2c_pins = i2c_pins.lock().await;
            let i2c_pins = i2c_pins.deref_mut();
            let mut sda = Flex::new(&mut i2c_pins.sda);
            sda.set_as_input_output(Pull::None, OutputDrive::Standard0Disconnect1);
            let mut i2c = bitbang::i2c::BitbangI2C::new(
                Output::new(&mut i2c_pins.scl, Level::High, OutputDrive::Standard0Disconnect1),
                sda,
                Default::default(),
            );

            let mut bme = bme280::Bme280::new_primary(&mut i2c);
            let bme_config = bme280::Configuration::default()
                .with_humidity_oversampling(bme280::Oversampling::Oversampling8X)
                .with_temperature_oversampling(bme280::Oversampling::Oversampling8X)
                .with_pressure_oversampling(bme280::Oversampling::Oversampling8X)
                .with_iir_filter(bme280::IIRFilter::Coefficient8);

            bme.init(bme_config).await?;
            Timer::after(Duration::from_millis(10)).await;

            // ignore these measurements (pressure is always around 600_000, and the next one 1m)
            let _measurements = bme.measure().await?;
            Timer::after(Duration::from_millis(100)).await;

            let measurements = bme.measure().await?;

            bme.set_mode(BME280_SLEEP_MODE).await?;

            measurements
        };

        info!(
            "BME: t={}, h={}, p={}",
            measurements.temperature, measurements.humidity, measurements.pressure
        );

        let temperature = measurements.temperature.as_temp();
        let humidity = measurements.humidity.as_humidity();
        let pressure = measurements.pressure.as_pressure();

        notify_all!(
            BME_EVENT_PROCESSOR,
            server.bme280,
            temperature = &temperature,
            humidity = &humidity,
            pressure = &pressure
        );

        Timer::after(BME_EVENT_PROCESSOR.get_timeout_duration()).await;
    }
}

async fn read_accel_task(
    i2c_pins: Arc<Mutex<ThreadModeRawMutex, BitbangI2CPins>>,
    server: &BleServer,
) -> Result<(), accelerometer::Error<bitbang::i2c::BitbangI2CError>> {
    loop {
        let _token = ACCELEROMETER_EVENT_PROCESSOR.wait_for_condition().await;
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

            let mut lis = Lis2dh12::new(i2c, SlaveAddr::Default).await?;
            lis.reset().await?;
            lis.set_odr(Odr::Hz50).await?;
            lis.set_bdu(true).await?;
            lis.set_fs(FullScale::G2).await?;
            lis.set_mode(crate::common::device::lis2dh12::reg::Mode::Normal).await?;
            lis.enable_axis((true, true, true)).await?;
            lis.enable_temp(true).await?;
            lis.enable_fifo(true).await?;
            lis.set_fm(FifoMode::Bypass).await?;

            lis.accel_norm().await?
        };

        info!("LIS: x={}, y={}, z={}", measurements.x, measurements.y, measurements.z);

        notify_all!(
            ACCELEROMETER_EVENT_PROCESSOR,
            server.accelerometer,
            x = &measurements.x,
            y = &measurements.y,
            z = &measurements.z
        );

        Timer::after(ACCELEROMETER_EVENT_PROCESSOR.get_timeout_duration()).await;
    }
}

async fn read_veml_task(
    i2c_pins: Arc<Mutex<ThreadModeRawMutex, BitbangI2CPins>>,
    server: &BleServer,
) -> Result<(), veml6040::Error<bitbang::i2c::BitbangI2CError>> {
    loop {
        let _token = COLOR_EVENT_PROCESSOR.wait_for_condition().await;

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

            // veml.enable().await?;
            // veml.set_measurement_mode(MeasurementMode::Auto).await?;
            // veml.set_integration_time(IntegrationTime::_40ms).await?;
            // does not work :(
            veml.write_config(
                0x00, // VEML6040_IT_40MS + VEML6040_AF_AUTO + VEML6040_SD_ENABLE
            )
            .await?;

            Timer::after(Duration::from_millis(40)).await;

            veml.read_all_channels().await?
        };
        info!("Color: {}", measurements);

        notify_all!(
            COLOR_EVENT_PROCESSOR,
            server.color,
            red = &measurements.red,
            green = &measurements.green,
            blue = &measurements.blue,
            white = &measurements.white
        );

        Timer::after(COLOR_EVENT_PROCESSOR.get_timeout_duration()).await;
    }
}
