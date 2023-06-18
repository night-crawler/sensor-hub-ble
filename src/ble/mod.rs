use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use lazy_static::lazy_static;
use nrf_softdevice::ble::Connection;

use crate::common::ble::custom_static_cell::CustomStaticCell;
use crate::common::ble::event_processor::EventProcessor;
use crate::common::ble::services::{
    AdcServiceEvent, BleServer, Bme280ServiceEvent, DeviceInformationServiceEvent,
};

pub(crate) mod conv;
pub(crate) mod custom_static_cell;
pub(crate) mod event_processor;
pub(crate) mod helper_macro;
pub(crate) mod services;
pub(crate) mod softdevice;
pub(crate) mod traits;

pub(crate) static SERVER: CustomStaticCell<BleServer> = CustomStaticCell::new();

pub(crate) static ADC_SERVICE_EVENTS: Channel<
    ThreadModeRawMutex,
    (Connection, AdcServiceEvent),
    1,
> = Channel::new();
pub(crate) static BME_SERVICE_EVENTS: Channel<
    ThreadModeRawMutex,
    (Connection, Bme280ServiceEvent),
    1,
> = Channel::new();
pub(crate) static DI_SERVICE_EVENTS: Channel<
    ThreadModeRawMutex,
    (Connection, DeviceInformationServiceEvent),
    1,
> = Channel::new();

lazy_static! {
    pub(crate) static ref NOTIFICATION_SETTINGS: EventProcessor = EventProcessor::default();
}
