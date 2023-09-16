use core::ops::DerefMut;
use defmt::info;
use embassy_nrf::gpio::{AnyPin, Level, Output, OutputDrive};
use embassy_nrf::{peripherals, spim};
use embassy_nrf::spim::{Spim};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use rclite::Arc;

use crate::common::device::device_manager::{ExpanderPins, Irqs};

trait Expander {
    fn select(&mut self, num: u8);
}

impl Expander for [Output<'_, AnyPin>; 3] {
    fn select(&mut self, num: u8) {
        let flags = [
            num & (1 << 0) != 0,
            num & (1 << 1) != 0,
            num & (1 << 2) != 0,
        ];
        self.iter_mut().zip(flags).for_each(|(pin, flag)| {
            if flag {
                pin.set_high();
            } else {
                pin.set_low();
            }
        });
    }
}

#[embassy_executor::task]
pub(crate) async fn expander_task(
    pins: Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
) {
    loop {
        let mut pins = pins.lock().await;
        let mut pins = pins.deref_mut();

        let mut cs_pins = [
            Output::new(&mut pins.a0, Level::Low, OutputDrive::HighDrive),
            Output::new(&mut pins.a1, Level::Low, OutputDrive::HighDrive),
            Output::new(&mut pins.a2, Level::Low, OutputDrive::HighDrive),
        ];
        cs_pins.select(2);

        let mut power = Output::new(&mut pins.power_switch, Level::High, OutputDrive::Standard);
        power.set_high();

        let mut spim_config = spim::Config::default();
        spim_config.frequency = pins.spim_config.frequency;
        spim_config.mode = pins.spim_config.mode;
        spim_config.orc = pins.spim_config.orc;

        Timer::after(Duration::from_millis(20)).await;
        let mut spi =
            Spim::new(&mut pins.spi_peripheral, Irqs, &mut pins.sck, &mut pins.miso, &mut pins.mosi, spim_config);

        let mut read_buf = [0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, ];
        let write_buf = read_buf.clone();
        match spi.transfer(&mut read_buf, &write_buf).await {
            Ok(_) => {
                info!("Expander: {:?}", read_buf);
            }
            Err(err) => {
                info!("Expander error: {:?}", err)
            }
        }

        power.set_low();

        Timer::after(Duration::from_secs(2)).await;
    }
}
