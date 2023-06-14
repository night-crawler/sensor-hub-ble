use core::ops::DerefMut;
use defmt::info;

use embassy_nrf::gpio::{Flex, Level, Output, OutputDrive, Pull};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use futures::{FutureExt, join};
use futures::select_biased;
use nrf_softdevice::ble::Connection;
use rclite::Arc;

use crate::ble_debug;
use crate::ble_notify;
use crate::common::bitbang;
use crate::common::ble::conv::ConvExt;
use crate::common::ble::SERVER;
use crate::common::ble::services::BleServer;
use crate::common::device::bme280;
use crate::common::device::bme280::Bme280Error;
use crate::common::device::device_manager::BitbangI2CPins;
use crate::common::device::lis2h12::{Lis2dh12, SlaveAddr};

#[embassy_executor::task]
pub(crate) async fn read_i2c0_task(i2c_pins: Arc<Mutex<ThreadModeRawMutex, BitbangI2CPins>>) {
    loop {
        let bme_fut = read_bme_task(Arc::clone(&i2c_pins), SERVER.get());
        let accel_fut = read_accel_task(Arc::clone(&i2c_pins), SERVER.get());

        let (bme, accel) = join!(bme_fut, accel_fut);
        if let Err(err) = bme {
            ble_debug!("Err: {}", err);
        }

        if let Err(err) = accel {
            ble_debug!("Err: {:?}", err);
        }

        Timer::after(Duration::from_millis(1000)).await;
    }
}


async fn read_bme_task(
    i2c_pins: Arc<Mutex<ThreadModeRawMutex, BitbangI2CPins>>,
    server: &BleServer,
) -> Result<(), Bme280Error> {
    loop {
        let measurements = {
            let mut i2c_pins = i2c_pins.lock().await;
            let i2c_pins = i2c_pins.deref_mut();
            let mut sda = Flex::new(&mut i2c_pins.sda);
            sda.set_as_input_output(Pull::None, OutputDrive::Standard);
            let mut i2c = bitbang::i2c::I2C::new(
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
            Timer::after(Duration::from_millis(100)).await;
            let measurements = bme.measure().await?;
            info!("t: {}, h: {}, p: {}", measurements.temperature, measurements.humidity, measurements.pressure);

            Timer::after(Duration::from_millis(100)).await;

            bme.measure().await?
        };

        let temperature = measurements.temperature.as_temp();
        let humidity = measurements.humidity.as_humidity();
        let pressure = measurements.pressure.as_pressure();

        for connection in Connection::iter() {
            ble_notify!(server.bme280, &connection, temp, &temperature);
            ble_notify!(server.bme280, &connection, humidity, &humidity);
            ble_notify!(server.bme280, &connection, pressure, &pressure);
        }

        Timer::after(Duration::from_millis(1000)).await;
    }
}


async fn read_accel_task(
    i2c_pins: Arc<Mutex<ThreadModeRawMutex, BitbangI2CPins>>,
    server: &BleServer,
) -> Result<(), accelerometer::Error<bitbang::i2c::Error>>{
    loop {
        {
            let mut i2c_pins = i2c_pins.lock().await;

            let i2c_pins = i2c_pins.deref_mut();
            let mut sda = Flex::new(&mut i2c_pins.sda);
            sda.set_as_input_output(Pull::None, OutputDrive::Standard);
            let mut i2c = bitbang::i2c::I2C::new(
                Output::new(&mut i2c_pins.scl, Level::High, OutputDrive::Standard),
                sda,
                Default::default(),
            );

            let mut lis = Lis2dh12::new(i2c, SlaveAddr::Default).await?;
            info!("Initialized lis");

            lis.set_mode(crate::common::device::lis2h12::reg::Mode::Normal).await?;
            info!("Set mode");
        }


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