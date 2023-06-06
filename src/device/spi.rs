use defmt::info;
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pull};
use embassy_nrf::peripherals::SPI3;
use embassy_nrf::spim;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use rclite::Arc;

use crate::common::device::device_manager::{EpdControlPins, SpiTxPins};
use crate::common::device::device_manager::Irqs;
use crate::common::device::epd::{buffer_len, Epd2in13};
use crate::common::device::epd::epd_controls::EpdControls;
use crate::common::device::epd::img::IMG;
use crate::common::device::epd::traits::{InternalWiAdditions, WaveshareDisplay};
use crate::common::device::error::CustomSpimError;

#[embassy_executor::task]
pub(crate) async fn epd_task(spi_pins: Arc<Mutex<ThreadModeRawMutex, SpiTxPins<SPI3>>>, control_pins: Arc<Mutex<ThreadModeRawMutex, EpdControlPins>>) {
    loop {
        info!("EPD task loop started");
        let mut spi_pins = spi_pins.lock().await;
        let mut control_pins = control_pins.lock().await;

        let result = draw_something(&mut spi_pins, &mut control_pins).await;

        match result {
            Ok(_) => {
                info!("Success!");
            }
            Err(e) => {
                info!("EPD Error: {:?}", e);
            }
        }

        Timer::after(Duration::from_secs(5)).await;
    }
}

async fn draw_something(spi_pins: &mut SpiTxPins<SPI3>, control_pins: &mut EpdControlPins) -> Result<(), CustomSpimError> {
    let mut config = spim::Config::default();
    config.frequency = spi_pins.config.frequency;
    config.mode = spi_pins.config.mode;
    config.orc = spi_pins.config.orc;

    let mut spi: spim::Spim<SPI3> = spim::Spim::new_txonly(
        &mut spi_pins.spim,
        Irqs,
        &mut spi_pins.sck,
        &mut spi_pins.mosi,
        config,
    );

    let busy = Input::new(&mut control_pins.busy, Pull::Up);
    let cs = Output::new(&mut control_pins.cs, Level::High, OutputDrive::Standard);
    let dc = Output::new(&mut control_pins.dc, Level::Low, OutputDrive::Standard);
    let rst = Output::new(&mut control_pins.rst, Level::Low, OutputDrive::Standard);
    let controls = EpdControls::new(
        &mut spi,
        busy,
        cs,
        dc,
        rst,
    );
    info!("Initialized EPD controls");

    let mut epd = Epd2in13::new(controls);
    epd.init().await?;

    info!("Initialized EPD");

    info!("Clearing frame");
    // epd.clear_frame().await?;
    info!("Cleared frame");


    epd.display(&IMG).await?;

    epd.sleep().await?;

    // epd.sleep().await?;
    info!("Updated and displayed frame");

    Ok(())
}