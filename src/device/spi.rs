use defmt::info;
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pull};
use embassy_nrf::peripherals::SPI2;
use embassy_nrf::spim;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use embedded_graphics_core::Drawable;
use embedded_graphics_core::prelude::{DrawTarget, Point};
use rclite::Arc;

use crate::common::device::device_manager::{EpdControlPins, SpiTxPins};
use crate::common::device::device_manager::Irqs;
use crate::common::device::epd::{Display2in13, Epd2in13};
use crate::common::device::epd::color::Color;
use crate::common::device::epd::epd_controls::EpdControls;
use crate::common::device::epd::graphics::DisplayRotation;
use crate::common::device::error::CustomSpimError;

#[embassy_executor::task]
pub(crate) async fn epd_task(spi_pins: Arc<Mutex<ThreadModeRawMutex, SpiTxPins<SPI2>>>, control_pins: Arc<Mutex<ThreadModeRawMutex, EpdControlPins>>) {
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

async fn draw_something(spi_pins: &mut SpiTxPins<SPI2>, control_pins: &mut EpdControlPins) -> Result<(), CustomSpimError> {
    let mut config = spim::Config::default();
    config.frequency = spi_pins.config.frequency;
    config.mode = spi_pins.config.mode;
    config.orc = spi_pins.config.orc;

    let mut spi: spim::Spim<SPI2> = spim::Spim::new_txonly(
        &mut spi_pins.spim,
        Irqs,
        &mut spi_pins.sck,
        &mut spi_pins.mosi,
        config,
    );

    let busy = Input::new(&mut control_pins.busy, Pull::Down);
    let cs = Output::new(&mut control_pins.cs, Level::High, OutputDrive::Standard);
    let dc = Output::new(&mut control_pins.dc, Level::Low, OutputDrive::Standard);
    let rst = Output::new(&mut control_pins.rst, Level::High, OutputDrive::Standard);
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
    epd.clear(Color::White).await?;
    info!("Cleared frame");

    let mut display = Display2in13::default();
    display.clear(Color::White).unwrap();
    display.set_rotation(DisplayRotation::Rotate0);
    draw_text(&mut display, "Rotate 0!", 5, 50);

    display.set_rotation(DisplayRotation::Rotate90);
    draw_text(&mut display, "Rotate 90!", 5, 50);

    display.set_rotation(DisplayRotation::Rotate180);
    draw_text(&mut display, "Rotate 180!", 5, 50);

    display.set_rotation(DisplayRotation::Rotate270);
    draw_text(&mut display, "Rotate 270!", 5, 50);

    epd.display(&display.buffer()).await?;

    epd.sleep().await?;

    // epd.sleep().await?;
    info!("Updated and displayed frame");

    Ok(())
}

fn draw_text(display: &mut Display2in13, text: &str, x: i32, y: i32) {
    let style = embedded_graphics::mono_font::MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
        .text_color(Color::Black)
        .background_color(Color::White)
        .build();

    let text_style = embedded_graphics::text::TextStyleBuilder::new().baseline(embedded_graphics::text::Baseline::Top).build();

    let _ = embedded_graphics::text::Text::with_text_style(text, Point::new(x, y), style, text_style).draw(display);
}