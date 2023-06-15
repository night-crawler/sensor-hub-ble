use core::cmp::min;
use core::fmt;
use core::str::from_utf8_unchecked;

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use nrf_softdevice::ble::Connection;

use crate::common::ble::SERVER;
use crate::common::device::config::{BLE_DEBUG_ARRAY_LEN, BLE_DEBUG_QUEUE_LEN};
use crate::common::device::error::DeviceError;

static CHANNEL: Channel<ThreadModeRawMutex, [u8; BLE_DEBUG_ARRAY_LEN], BLE_DEBUG_QUEUE_LEN> = Channel::new();

#[embassy_executor::task]
pub(crate) async fn ble_debug_notify_task() {
    let server = SERVER.get();

    loop {
        let message = CHANNEL.recv().await;
        for connection in Connection::iter() {
            if server.dis.debug_notify(&connection, &message).is_err() {
                let _ = server.dis.debug_set(&message);
            }
        }
    }
}


pub struct WriteTo<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl<'a> WriteTo<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        WriteTo { buf, len: 0 }
    }

    pub fn to_str(self) -> Option<&'a str> {
        if self.len <= self.buf.len() {
            Some(unsafe { from_utf8_unchecked(&self.buf[..self.len]) })
        } else {
            None
        }
    }
}

impl<'a> fmt::Write for WriteTo<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.len > self.buf.len() {
            return Err(fmt::Error);
        }

        let rem = &mut self.buf[self.len..];
        let raw_s = s.as_bytes();
        let num = min(raw_s.len(), rem.len());

        rem[..num].copy_from_slice(&raw_s[..num]);
        self.len += raw_s.len();

        if num < raw_s.len() {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

pub fn ble_debug_format<'a>(arg: fmt::Arguments) -> Result<(), DeviceError> {
    let mut buf = [0u8; 64];
    let mut w = WriteTo::new(&mut buf);
    fmt::write(&mut w, arg)?;
    CHANNEL.try_send(buf)?;
    Ok(())
}

#[macro_export]
macro_rules! ble_debug {
    (
        $($t:tt)*
    ) => {{
        let _ = $crate::common::device::ble_debugger::ble_debug_format(format_args!($($t)*));
    }};
}
