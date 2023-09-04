use core::ops::Deref;
use defmt::info;
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pull};
use embassy_nrf::peripherals::SPI2;
use embassy_nrf::spim;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use embedded_graphics_core::Drawable;
use embedded_graphics_core::prelude::DrawTarget;
use rclite::Arc;
use spim::Spim;

use crate::common::device::device_manager::{EpdControlPins, SpiTxPins};
use crate::common::device::device_manager::Irqs;
use crate::common::device::epd::{Display2in13, Epd2in13};
use crate::common::device::epd::color::Color;
use crate::common::device::epd::epd_controls::EpdControls;
use crate::common::device::epd::graphics::DisplayRotation;
use crate::common::device::ui::device_ui::Ui;

use crate::common::device::ui::error::UiError;
use crate::common::device::ui::text_repr::TextRepr;
use crate::common::device::ui::UI_STORE;

#[embassy_executor::task]
pub(crate) async fn epd_task(
    spi_pins: Arc<Mutex<ThreadModeRawMutex, SpiTxPins<SPI2>>>,
    control_pins: Arc<Mutex<ThreadModeRawMutex, EpdControlPins>>,
) {
    loop {
        info!("EPD task loop started");
        Timer::after(Duration::from_secs(20)).await;

        let mut spi_pins = spi_pins.lock().await;
        let mut control_pins = control_pins.lock().await;

        let result = draw_ui(&mut spi_pins, &mut control_pins).await;

        match result {
            Ok(_) => {
                info!("Success!");
            }
            Err(e) => {
                info!("EPD Error: {:?}", e);
            }
        }

        Timer::after(Duration::from_secs(5000000)).await;
    }
}


async fn draw_ui(
    spi_pins: &mut SpiTxPins<SPI2>,
    control_pins: &mut EpdControlPins,
) -> Result<(), UiError<<Display2in13 as DrawTarget>::Error>> {
    let mut config = spim::Config::default();
    config.frequency = spi_pins.config.frequency;
    config.mode = spi_pins.config.mode;
    config.orc = spi_pins.config.orc;

    let mut spi: Spim<SPI2> =
        Spim::new_txonly(&mut spi_pins.spim, Irqs, &mut spi_pins.sck, &mut spi_pins.mosi, config);

    let busy = Input::new(&mut control_pins.busy, Pull::Down);
    let cs = Output::new(&mut control_pins.cs, Level::High, OutputDrive::Standard);
    let dc = Output::new(&mut control_pins.dc, Level::Low, OutputDrive::Standard);
    let rst = Output::new(&mut control_pins.rst, Level::High, OutputDrive::Standard);
    let controls = EpdControls::new(&mut spi, busy, cs, dc, rst);
    info!("Initialized EPD controls");

    let mut epd: Epd2in13<spim::Error, _> = Epd2in13::new(controls);
    epd.init().await?;

    info!("Initialized EPD");

    let mut display = Display2in13::default();
    display.set_rotation(DisplayRotation::Rotate90);

    let mut ui = Ui::new(&mut display, Color::Black, Color::White);
    let text_repr  = {
        let store = UI_STORE.lock().await;
        TextRepr::from(store.deref())
    };
    ui.draw(text_repr)?;

    epd.display(display.buffer()).await?;
    epd.sleep().await?;

    Ok(())
}
