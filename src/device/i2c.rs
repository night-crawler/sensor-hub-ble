use embassy_nrf::peripherals::TWISPI0;
use embassy_nrf::twim::{Config, Twim};
use embassy_time::{Duration, Timer};
use nrf_softdevice::ble::Connection;

use crate::common::ble::services::BleServer;
use crate::common::device::device_manager::{I2CPins, Irqs};

pub(crate) async fn read_i2c0<'a>(i2c_pins: &mut I2CPins<TWISPI0>, server: &'a BleServer, connection: &'a Connection) {
    let mut config = Config::default();
    config.frequency = i2c_pins.config.frequency;
    config.sda_high_drive = i2c_pins.config.sda_high_drive;
    config.sda_pullup = i2c_pins.config.sda_pullup;
    config.scl_high_drive = i2c_pins.config.scl_high_drive;
    config.scl_pullup = i2c_pins.config.scl_pullup;
    let mut twim = Twim::new(&mut i2c_pins.twim, Irqs, &mut i2c_pins.sda, &mut i2c_pins.scl, config);

    loop {
        Timer::after(Duration::from_millis(1000)).await;
    }

    // twim.write()
    // let q = twim.write_read().await;
}
