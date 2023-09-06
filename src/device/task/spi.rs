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
use futures::FutureExt;
use futures::select_biased;
use rclite::Arc;
use spim::Spim;
use crate::common::ble::trigger_all_sensor_update;
use crate::common::device::config::ALL_TASK_COMPLETION_INTERVAL;

use crate::common::device::device_manager::{EpdControlPins, SpiTxPins};
use crate::common::device::device_manager::Irqs;
use crate::common::device::epd::{Display2in13, Epd2in13};
use crate::common::device::epd::color::Color;
use crate::common::device::epd::epd_controls::EpdControls;
use crate::common::device::epd::graphics::DisplayRotation;
use crate::common::device::ui::controls::DisplayRefreshType;
use crate::common::device::ui::device_ui::Ui;
use crate::common::device::ui::DISPLAY_REFRESH_EVENTS;
use crate::common::device::ui::error::UiError;
use crate::common::device::ui::text_repr::TextRepr;
use crate::common::device::ui::UI_STORE;

#[embassy_executor::task]
pub(crate) async fn epd_task(
    spi_pins: Arc<Mutex<ThreadModeRawMutex, SpiTxPins<SPI2>>>,
    control_pins: Arc<Mutex<ThreadModeRawMutex, EpdControlPins>>,
) {
    let mut refresh_type = DisplayRefreshType::Full;

    // color oversampling takes 40ms * 50
    Timer::after(ALL_TASK_COMPLETION_INTERVAL).await;

    loop {
        info!("Refreshing display");
        let mut spi_pins = spi_pins.lock().await;
        let mut control_pins = control_pins.lock().await;

        let result = draw_ui(&mut spi_pins, &mut control_pins, refresh_type).await;
        refresh_type = DisplayRefreshType::Full;

        match result {
            Ok(_) => {
                info!("Success!");
            }
            Err(e) => {
                info!("EPD Error: {:?}", e);
            }
        }

        select_biased! {
            _ = Timer::after(Duration::from_secs(300)).fuse() => {}
            next_refresh_type = DISPLAY_REFRESH_EVENTS.recv().fuse() => {
                info!("Received refresh event: {:?}", next_refresh_type);
                refresh_type = next_refresh_type;
            }
        }
    }
}


async fn draw_ui(
    spi_pins: &mut SpiTxPins<SPI2>,
    control_pins: &mut EpdControlPins,
    refresh_type: DisplayRefreshType,
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
    let text_repr = {
        let store = UI_STORE.lock().await;
        TextRepr::from(store.deref())
    };
    ui.draw(text_repr)?;

    match refresh_type {
        DisplayRefreshType::Partial => {
            epd.display_partial(display.buffer()).await?;
        }
        DisplayRefreshType::Full => {
            epd.display(display.buffer()).await?;
        }
    }

    epd.sleep().await?;

    Ok(())
}
