use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use nrf_softdevice::ble::Connection;

use crate::common::ble::event_processor::{
    AccelerometerNotificationSettings, AdcNotificationSettings, BmeNotificationSettings,
    ColorNotificationSettings, DiNotificationSettings, EventProcessor,
};
use crate::common::ble::services::{AccelerometerServiceEvent, AdcServiceEvent, BleServer, Bme280ServiceEvent, ColorServiceEvent, DeviceInformationServiceEvent, ExpanderServiceEvent};
use crate::common::device::config::NUM_CONNECTIONS;
use crate::common::device::persistence::flash_manager::FlashManager;
use crate::common::util::custom_static_cell::CustomStaticCell;

pub(crate) mod conv;
pub(crate) mod event_processor;
pub(crate) mod helper_macro;
pub(crate) mod services;
pub(crate) mod softdevice;
pub(crate) mod traits;

pub(crate) static SERVER: CustomStaticCell<BleServer> = CustomStaticCell::new();
pub(crate) static FLASH_MANAGER: CustomStaticCell<FlashManager> = CustomStaticCell::new();

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

pub(crate) static SPI_EXPANDER_EVENTS: Channel<
    ThreadModeRawMutex,
    (Connection, ExpanderServiceEvent),
    10,
> = Channel::new();

pub(crate) static SPI_EXPANDER_LOCK_OWNER: Mutex<ThreadModeRawMutex, Option<Connection>> = Mutex::new(None);

pub(crate) static DEVICE_EVENT_PROCESSOR: EventProcessor<
    DiNotificationSettings,
    DeviceInformationServiceEvent,
    2,
> = EventProcessor::new(Some("device"));
pub(crate) static BME_EVENT_PROCESSOR: EventProcessor<
    BmeNotificationSettings,
    Bme280ServiceEvent,
    1,
> = EventProcessor::new(Some("bme280"));
pub(crate) static ADC_EVENT_PROCESSOR: EventProcessor<AdcNotificationSettings, AdcServiceEvent, 1> =
    EventProcessor::new(Some("adc"));
pub(crate) static ACCELEROMETER_EVENT_PROCESSOR: EventProcessor<
    AccelerometerNotificationSettings,
    AccelerometerServiceEvent,
    1,
> = EventProcessor::new(Some("accelerometer"));
pub(crate) static COLOR_EVENT_PROCESSOR: EventProcessor<
    ColorNotificationSettings,
    ColorServiceEvent,
    1,
> = EventProcessor::new(Some("color"));


pub(crate) fn trigger_all_sensor_update() {
    // Fire event twice, since one event will be consumed by NRF temperature task
    // and one will go to the battery task
    DEVICE_EVENT_PROCESSOR.fire_once();
    DEVICE_EVENT_PROCESSOR.fire_once();

    BME_EVENT_PROCESSOR.fire_once();
    ADC_EVENT_PROCESSOR.fire_once();
    ACCELEROMETER_EVENT_PROCESSOR.fire_once();
    COLOR_EVENT_PROCESSOR.fire_once();
}
