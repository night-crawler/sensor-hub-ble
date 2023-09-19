use core::fmt;

use defmt::info;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use nrf_softdevice::ble::Connection;

use crate::common::ble::SERVER;
use crate::common::device::config::{BLE_DEBUG_ARRAY_LEN, BLE_DEBUG_QUEUE_LEN};
use crate::common::device::error::DeviceError;
use crate::common::util::buf_writer::WriteTo;
use crate::DEVICE_EVENT_PROCESSOR;
use crate::notify_all;

static CHANNEL: Channel<ThreadModeRawMutex, (Option<Connection>, [u8; BLE_DEBUG_ARRAY_LEN]), BLE_DEBUG_QUEUE_LEN> =
    Channel::new();


pub(crate) trait ConnectionDebug {
    fn debug(&self, args: fmt::Arguments);
}

impl ConnectionDebug for Connection {
    fn debug(&self, args: fmt::Arguments) {
        let _ = ble_debug_push(Some(self.clone()), args);
    }
}

#[embassy_executor::task]
pub(crate) async fn ble_debug_notify_task() {
    let server = SERVER.get();

    loop {
        let (connection, message) = CHANNEL.recv().await;
        if let Some(connection) = connection {
            let _ = SERVER.get().dis.debug_notify(&connection, &message);
        } else {
            notify_all!(DEVICE_EVENT_PROCESSOR, server.dis, debug = &message);
        }
    }
}


pub fn ble_debug_push(connection: Option<Connection>, args: fmt::Arguments) -> Result<(), DeviceError> {
    let mut buf = [0u8; BLE_DEBUG_ARRAY_LEN];
    let mut w = WriteTo::new(&mut buf);
    fmt::write(&mut w, args)?;
    info!("ble_debug: {}", w.to_str().unwrap_or("invalid utf8"));
    if let Err(err) = CHANNEL.try_send((connection, buf)) {
        info!("Failed to put debug message to the channel");
    }
    Ok(())
}

#[macro_export]
macro_rules! ble_debug {
    (
        $($t:tt)*
    ) => {{
        let _ = $crate::common::util::ble_debugger::ble_debug_push(None, format_args!($($t)*));
    }};
}
