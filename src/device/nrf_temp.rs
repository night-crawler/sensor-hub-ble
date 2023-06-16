use core::sync::atomic::Ordering;

use embassy_time::{Duration, Timer};
use nrf_softdevice::ble::Connection;
use nrf_softdevice::{temperature_celsius, Softdevice};

use crate::common::ble::conv::ConvExt;
use crate::common::ble::services::BleServer;
use crate::common::device::adc::ADC_TIMEOUT;
use crate::common::device::led_animation::{LedState, LedStateAnimation};

pub(crate) async fn notify_nrf_temp<'a>(
    sd: &Softdevice,
    server: &'a BleServer,
    connection: &'a Connection,
) {
    loop {
        let value = match temperature_celsius(sd) {
            Ok(value) => value.to_num::<f32>().as_temp(),
            Err(_) => {
                LedStateAnimation::blink_long(&[LedState::Red]);
                continue;
            }
        };
        match server.dis.temp_notify(connection, &value) {
            Ok(_) => {}
            Err(_) => {
                let _ = server.dis.temp_set(&value);
            }
        }
        Timer::after(Duration::from_millis(
            ADC_TIMEOUT.load(Ordering::Relaxed) as u64
        ))
        .await;
    }
}
