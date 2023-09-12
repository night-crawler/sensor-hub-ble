use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use futures::{FutureExt, select_biased};
use rclite::Arc;

use crate::ble_debug;
use crate::common::bitbang;
use crate::common::bitbang::shared_i2c::SharedBitbangI2cPins;
use crate::common::ble::{
    ACCELEROMETER_EVENT_PROCESSOR, BME_EVENT_PROCESSOR, COLOR_EVENT_PROCESSOR, SERVER,
};
use crate::common::ble::conv::ConvExt;
use crate::common::ble::services::BleServer;
use crate::common::device::{bme280, veml6040};
use crate::common::device::bme280::{BME280_SLEEP_MODE, Bme280Error};
use crate::common::device::device_manager::BitbangI2CPins;
use crate::common::device::lis2dh12::{Lis2dh12, SlaveAddr};
use crate::common::device::lis2dh12::reg::{FifoMode, FullScale, Odr};
use crate::common::device::ui::UI_STORE;
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
                    ble_debug!("BME error: {}", err);
                }
            },
            result = accel_fut.fuse() => {
                if let Err(err) = result {
                    ble_debug!("Accel error: {:?}", err);
                }
            },
            result = color_fut.fuse() => {
                if let Err(err) = result {
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
            let mut i2c = SharedBitbangI2cPins::new(i2c_pins.as_ref());

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

        {
            let mut store = UI_STORE.lock().await;
            store.temperature = measurements.temperature;
            store.humidity = measurements.humidity;
            store.pressure = measurements.pressure;
        }


        // info!(
        //     "BME: t={}, h={}, p={}",
        //     measurements.temperature, measurements.humidity, measurements.pressure
        // );

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
            let i2c = SharedBitbangI2cPins::new(i2c_pins.as_ref());

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

            let measurements = lis.accel_norm().await?;
            lis.set_mode(crate::common::device::lis2dh12::reg::Mode::LowPower).await?;
            lis.set_odr(Odr::PowerDown).await?;

            measurements
        };

        {
            let mut store = UI_STORE.lock().await;
            store.x = measurements.x;
            store.y = measurements.y;
            store.z = measurements.z;
        }

        // info!("LIS: x={}, y={}, z={}", measurements.x, measurements.y, measurements.z);

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
            let i2c = SharedBitbangI2cPins::new(i2c_pins.as_ref());
            let mut veml = veml6040::Veml6040::new(i2c);
            veml.set_measurement_mode(veml6040::MeasurementMode::Auto).await?;
            veml.read_all_channels_with_oversampling(veml6040::IntegrationTime::_1280ms, 2).await?
        };

        let ambient = measurements.ambient_light(veml6040::IntegrationTime::_1280ms);
        let cct = measurements.compute_cct().unwrap_or(0.0f32) as u16;

        {
            let mut store = UI_STORE.lock().await;
            store.cct = cct;
            store.lux = ambient;
            store.r = measurements.red;
            store.g = measurements.green;
            store.b = measurements.blue;
            store.w = measurements.white;
        }

        // info!("Color: {}; ambient light: {}, cct: {}", measurements, ambient, cct);

        notify_all!(
            COLOR_EVENT_PROCESSOR,
            server.color,
            red = &measurements.red,
            green = &measurements.green,
            blue = &measurements.blue,
            white = &measurements.white,
            cct = &cct,
            lux = &ambient.as_luminous_flux()
        );

        Timer::after(COLOR_EVENT_PROCESSOR.get_timeout_duration()).await;
    }
}
