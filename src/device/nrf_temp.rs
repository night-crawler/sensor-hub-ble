use embassy_time::Timer;
use nrf_softdevice::ble::Connection;
use nrf_softdevice::{temperature_celsius, Softdevice};

use crate::ble_debug;
use crate::common::ble::conv::ConvExt;
use crate::common::ble::{NOTIFICATION_SETTINGS, SERVER};

#[embassy_executor::task]
pub(crate) async fn notify_nrf_temp(sd: &'static Softdevice) {
    loop {
        let value = match temperature_celsius(sd) {
            Ok(value) => value.to_num::<f32>().as_temp(),
            Err(e) => {
                ble_debug!("Failed to measure temp: {:?}", e);
                continue;
            }
        };

        let server = SERVER.get();
        for connection in Connection::iter() {
            if let Err(_) = server.dis.temperature_notify(&connection, &value) {
                let _ = server.dis.temperature_set(&value);
            }
        }

        Timer::after(NOTIFICATION_SETTINGS.get_di_timeout_duration()).await;
    }
}
