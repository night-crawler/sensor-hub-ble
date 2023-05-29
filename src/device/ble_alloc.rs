use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use lazy_static::lazy_static;
use nrf_softdevice::ble::Connection;

use crate::common::device::config::NUM_CONNECTIONS;

lazy_static! {
    pub static ref CONNECTIONS: Mutex<ThreadModeRawMutex, [Option<Connection>; NUM_CONNECTIONS]> = {
        Mutex::new(Default::default())
    };
}
