use crate::common::ble::custom_static_cell::CustomStaticCell;
use crate::common::ble::services::BleServer;

pub(crate) mod softdevice;
pub(crate) mod services;
pub(crate) mod conv;
pub(crate) mod custom_static_cell;

pub(crate) static SERVER: CustomStaticCell<BleServer> = CustomStaticCell::new();
