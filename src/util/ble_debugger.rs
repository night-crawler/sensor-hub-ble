use core::fmt;
use defmt::info;

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

use crate::common::ble::SERVER;
use crate::common::device::config::{BLE_DEBUG_ARRAY_LEN, BLE_DEBUG_QUEUE_LEN};
use crate::common::device::error::DeviceError;
use crate::common::util::buf_writer::WriteTo;
use crate::notify_all;
use crate::DEVICE_EVENT_PROCESSOR;

static CHANNEL: Channel<ThreadModeRawMutex, [u8; BLE_DEBUG_ARRAY_LEN], BLE_DEBUG_QUEUE_LEN> =
    Channel::new();

#[embassy_executor::task]
pub(crate) async fn ble_debug_notify_task() {
    let server = SERVER.get();

    loop {
        let message = CHANNEL.recv().await;
        notify_all!(DEVICE_EVENT_PROCESSOR, server.dis, debug = &message);
    }
}


pub fn ble_debug_format(arg: fmt::Arguments) -> Result<(), DeviceError> {
    let mut buf = [0u8; 64];
    let mut w = WriteTo::new(&mut buf);
    fmt::write(&mut w, arg)?;
    info!("ble_debug: {}", w.to_str().unwrap_or("invalid utf8"));
    CHANNEL.try_send(buf)?;
    Ok(())
}

#[macro_export]
macro_rules! ble_debug {
    (
        $($t:tt)*
    ) => {{
        let _ = $crate::common::util::ble_debugger::ble_debug_format(format_args!($($t)*));
    }};
}
