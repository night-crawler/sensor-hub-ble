use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use lazy_static::lazy_static;
use nrf_softdevice::ble::Connection;

use crate::common::ble::event_processor::EventProcessor;
use crate::common::ble::services::{
    AdcServiceEvent, BleServer, Bme280ServiceEvent, DeviceInformationServiceEvent,
};
use crate::common::device::config::NUM_CONNECTIONS;
use crate::common::util::condition::Condition;
use crate::common::util::custom_static_cell::CustomStaticCell;

pub(crate) mod conv;
pub(crate) mod event_processor;
pub(crate) mod helper_macro;
pub(crate) mod services;
pub(crate) mod softdevice;
pub(crate) mod traits;

pub(crate) static SERVER: CustomStaticCell<BleServer> = CustomStaticCell::new();

pub(crate) static ADC_SERVICE_EVENTS: Channel<
    ThreadModeRawMutex,
    (Connection, AdcServiceEvent),
    NUM_CONNECTIONS,
> = Channel::new();

pub(crate) static BME_SERVICE_EVENTS: Channel<
    ThreadModeRawMutex,
    (Connection, Bme280ServiceEvent),
    NUM_CONNECTIONS,
> = Channel::new();

pub(crate) static DI_SERVICE_EVENTS: Channel<
    ThreadModeRawMutex,
    (Connection, DeviceInformationServiceEvent),
    NUM_CONNECTIONS,
> = Channel::new();

pub(crate) static BME_TASK_CONDITION: Condition = Condition::new();

lazy_static! {
    pub(crate) static ref NOTIFICATION_SETTINGS: EventProcessor = EventProcessor::default();
}
