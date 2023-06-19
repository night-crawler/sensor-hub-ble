use embassy_time::Timer;
use nrf_softdevice::ble::Connection;
use nrf_softdevice::{temperature_celsius, Softdevice};

use crate::common::ble::conv::ConvExt;
use crate::common::ble::{DEVICE_EVENT_PROCESSOR, SERVER};
use crate::{ble_debug, notify_all};

#[embassy_executor::task]
pub(crate) async fn notify_nrf_temp(sd: &'static Softdevice) {
    loop {
        let _token = DEVICE_EVENT_PROCESSOR.wait_for_condition().await;
        let value = match temperature_celsius(sd) {
            Ok(value) => value.to_num::<f32>().as_temp(),
            Err(e) => {
                ble_debug!("Failed to measure temp: {:?}", e);
                continue;
            }
        };

        let server = SERVER.get();

        notify_all!(DEVICE_EVENT_PROCESSOR, server.dis, temperature = &value);

        Timer::after(DEVICE_EVENT_PROCESSOR.get_timeout_duration()).await;
    }
}
