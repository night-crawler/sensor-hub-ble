use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use nrf_softdevice::ble::Connection;

use crate::common::ble::event_processor::{
    AccelerometerNotificationSettings, AdcNotificationSettings, BmeNotificationSettings,
    ColorNotificationSettings, DiNotificationSettings, EventProcessor,
};
use crate::common::ble::services::{
    AccelerometerServiceEvent, AdcServiceEvent, BleServer, Bme280ServiceEvent, ColorServiceEvent,
    DeviceInformationServiceEvent,
};
use crate::common::device::config::NUM_CONNECTIONS;
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

pub(crate) static ACCELEROMETER_SERVICE_EVENTS: Channel<
    ThreadModeRawMutex,
    (Connection, AccelerometerServiceEvent),
    NUM_CONNECTIONS,
> = Channel::new();

pub(crate) static COLOR_SERVICE_EVENTS: Channel<
    ThreadModeRawMutex,
    (Connection, ColorServiceEvent),
    NUM_CONNECTIONS,
> = Channel::new();

pub(crate) static DEVICE_EVENT_PROCESSOR: EventProcessor<
    DiNotificationSettings,
    DeviceInformationServiceEvent,
    2,
> = EventProcessor::new();
pub(crate) static BME_EVENT_PROCESSOR: EventProcessor<
    BmeNotificationSettings,
    Bme280ServiceEvent,
    1,
> = EventProcessor::new();
pub(crate) static ADC_EVENT_PROCESSOR: EventProcessor<AdcNotificationSettings, AdcServiceEvent, 1> =
    EventProcessor::new();
pub(crate) static ACCELEROMETER_EVENT_PROCESSOR: EventProcessor<
    AccelerometerNotificationSettings,
    AccelerometerServiceEvent,
    1,
> = EventProcessor::new();
pub(crate) static COLOR_EVENT_PROCESSOR: EventProcessor<
    ColorNotificationSettings,
    ColorServiceEvent,
    1,
> = EventProcessor::new();
