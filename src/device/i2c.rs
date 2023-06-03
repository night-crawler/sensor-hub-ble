use embassy_nrf::peripherals::TWISPI0;
use embassy_nrf::twim;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use futures::FutureExt;
use futures::select_biased;
use nrf_softdevice::ble::Connection;

use crate::ble_debug;
use crate::ble_notify;
use crate::common::ble::conv::ConvExt;
use crate::common::ble::services::BleServer;
use crate::common::device::bme280;
use crate::common::device::device_manager::{I2CPins, Irqs};

pub(crate) async fn read_i2c0<'a>(i2c_pins: &mut I2CPins<TWISPI0>, server: &'a BleServer, connection: &'a Connection) {
    loop {
        let mut twim_config = twim::Config::default();
        twim_config.frequency = i2c_pins.config.frequency;
        twim_config.sda_high_drive = i2c_pins.config.sda_high_drive;
        twim_config.sda_pullup = i2c_pins.config.sda_pullup;
        twim_config.scl_high_drive = i2c_pins.config.scl_high_drive;
        twim_config.scl_pullup = i2c_pins.config.scl_pullup;

        let twim: twim::Twim<TWISPI0> = twim::Twim::new(
            &mut i2c_pins.twim,
            Irqs,
            &mut i2c_pins.sda,
            &mut i2c_pins.scl,
            twim_config,
        );
        // drop it later!
        let twim: Mutex<ThreadModeRawMutex, twim::Twim<TWISPI0>> = Mutex::new(twim);

        select_biased! {
            res = read_bme_task(&twim, server, connection).fuse() => {
                if let Err(err) = res {
                    ble_debug!("Error: {:?}", err);
                }
            }
        }

        Timer::after(Duration::from_millis(1000)).await;
    }
}


async fn read_bme_task<'a>(
    twim: &Mutex<ThreadModeRawMutex, twim::Twim<'_, TWISPI0>>,
    server: &'a BleServer,
    connection: &'a Connection,
) -> Result<(), bme280::Bme280Error> {
    let mut bme = bme280::Bme280::new_primary(&twim);
    let bme_config = bme280::Configuration::default()
        .with_humidity_oversampling(bme280::Oversampling::Oversampling8X)
        .with_temperature_oversampling(bme280::Oversampling::Oversampling8X)
        .with_pressure_oversampling(bme280::Oversampling::Oversampling8X)
        .with_iir_filter(bme280::IIRFilter::Coefficient8);

    bme.init(bme_config).await?;
    Timer::after(Duration::from_millis(100)).await;
    let measurements = bme.measure().await?;
    ble_debug!("t: {}, h: {}, p: {}", measurements.temperature, measurements.humidity, measurements.pressure);

    Timer::after(Duration::from_millis(100)).await;

    loop {
        let measurements = bme.measure().await?;
        let temperature = measurements.temperature.as_temp();
        let humidity = measurements.humidity.as_humidity();
        let pressure = measurements.pressure.as_pressure();

        ble_notify!(server.bme280, connection, temp, &temperature);
        ble_notify!(server.bme280, connection, humidity, &humidity);
        ble_notify!(server.bme280, connection, pressure, &pressure);

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