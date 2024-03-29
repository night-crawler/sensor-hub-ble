use alloc::collections::BTreeMap;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU32, Ordering};

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Duration;
use nrf_softdevice::ble::Connection;

use crate::{
    impl_is_task_enabled, impl_read_event_channel, impl_set_notification,
    impl_settings_event_consumer, impl_timeout_event_characteristic,
};
use crate::common::ble::{ACCELEROMETER_EVENT_PROCESSOR, ACCELEROMETER_SERVICE_EVENTS, ADC_EVENT_PROCESSOR, ADC_SERVICE_EVENTS, BME_EVENT_PROCESSOR, BME_SERVICE_EVENTS, COLOR_EVENT_PROCESSOR, COLOR_SERVICE_EVENTS, DEVICE_EVENT_PROCESSOR, DI_SERVICE_EVENTS, FLASH_MANAGER};
use crate::common::ble::services::{
    AccelerometerServiceEvent, AdcServiceEvent, Bme280ServiceEvent, ColorServiceEvent,
    DeviceInformationServiceEvent,
};
use crate::common::ble::traits::{
    IsTaskEnabled, SettingsEventConsumer, TimeoutEventCharacteristic,
};
use crate::common::util::condition::{Condition, ConditionToken};

#[derive(Default, Clone)]
pub(crate) struct AccelerometerNotificationSettings {
    pub(crate) x: bool,
    pub(crate) y: bool,
    pub(crate) z: bool,
}

#[derive(Default, Clone)]
pub(crate) struct ColorNotificationSettings {
    pub(crate) red: bool,
    pub(crate) green: bool,
    pub(crate) blue: bool,
    pub(crate) white: bool,
    pub(crate) cct: bool,
    pub(crate) lux: bool,
}

#[derive(Default, Clone)]
pub(crate) struct AdcNotificationSettings {
    pub(crate) voltage0: bool,
    pub(crate) voltage1: bool,
    pub(crate) voltage2: bool,
    pub(crate) voltage3: bool,
    pub(crate) voltage4: bool,
    pub(crate) voltage5: bool,
    pub(crate) voltage6: bool,
    pub(crate) samples: bool,
    pub(crate) elapsed: bool,
}

#[derive(Default, Clone)]
pub(crate) struct BmeNotificationSettings {
    pub(crate) temperature: bool,
    pub(crate) humidity: bool,
    pub(crate) pressure: bool,
}

#[derive(Default, Clone)]
pub(crate) struct DiNotificationSettings {
    pub(crate) temperature: bool,
    pub(crate) battery_voltage: bool,
    pub(crate) debug: bool,
}

pub(crate) struct EventProcessor<S, E, const T: usize> {
    notification_settings: Mutex<ThreadModeRawMutex, BTreeMap<Connection, S>>,
    timeout: AtomicU32,
    condition: Condition<T>,
    _phantom_data: PhantomData<E>,
}

impl<S, E, const T: usize> EventProcessor<S, E, T>
    where
        S: Default + SettingsEventConsumer<E> + IsTaskEnabled + Clone,
        E: TimeoutEventCharacteristic,
{
    pub(crate) const fn new(name: Option<&'static str>) -> Self {
        Self {
            notification_settings: Mutex::new(BTreeMap::new()),
            timeout: AtomicU32::new(1000),
            condition: Condition::new(name),
            _phantom_data: PhantomData,
        }
    }

    pub(crate) async fn process_event(&self, connection: Connection, event: E) {
        if let Some(timeout) = event.get_timeout() {
            self.timeout.store(timeout, Ordering::SeqCst);
        }

        let mut settings_map = self.notification_settings.lock().await;
        let settings = settings_map.entry(connection).or_default();
        settings.consume(event).await;

        self.set_task_enabled_state(&settings_map);
    }

    pub(crate) fn get_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.timeout.load(Ordering::Relaxed) as u64)
    }

    fn set_task_enabled_state(&self, settings: &BTreeMap<Connection, S>) {
        let should_enable = settings.values().any(|settings| settings.is_task_enabled());
        self.condition.set(should_enable);
    }

    pub(crate) fn fire_once(&self) {
        self.condition.fire_once();
    }

    pub(crate) async fn register_connection(&self, connection: &Connection) {
        _ = self.notification_settings.lock().await.entry(connection.clone()).or_default();
    }

    pub(crate) async fn drop_connection(&self, connection: &Connection) {
        let mut settings_map = self.notification_settings.lock().await;
        settings_map.remove(connection);
        self.set_task_enabled_state(&settings_map);
    }

    pub(crate) async fn wait_for_condition(&self) -> ConditionToken<T> {
        self.condition.lock().await
    }

    pub(crate) async fn get_connection_settings(&self, connection: &Connection) -> Option<S> {
        self.notification_settings.lock().await.get(connection).cloned()
    }
}

impl_settings_event_consumer!(
    AdcNotificationSettings,
    AdcServiceEvent,
    Voltage0,
    Voltage1,
    Voltage2,
    Voltage3,
    Voltage4,
    Voltage5,
    Voltage6,
    Samples,
    Elapsed
);

impl_settings_event_consumer!(
    AccelerometerNotificationSettings,
    AccelerometerServiceEvent,
    X,
    Y,
    Z
);

impl_settings_event_consumer!(
    ColorNotificationSettings,
    ColorServiceEvent,
    Red,
    Green,
    Blue,
    White,
    Lux,
    Cct
);
impl SettingsEventConsumer<Bme280ServiceEvent> for BmeNotificationSettings {
    async fn consume(&mut self, event: Bme280ServiceEvent) {
        let mut next_calibration_data = match event {
            Bme280ServiceEvent::TemperatureCccdWrite { notifications } => {
                self.temperature = notifications;
                return;
            }
            Bme280ServiceEvent::HumidityCccdWrite { notifications } => {
                self.humidity = notifications;
                return;
            }
            Bme280ServiceEvent::PressureCccdWrite { notifications } => {
                self.pressure = notifications;
                return;
            }
            Bme280ServiceEvent::HumidityOffsetWrite(value) => {
                let mut data = FLASH_MANAGER.get().get_last_calibration_data().await;
                data.bme_humidity = f32::from_le_bytes(value);
                data
            }
            Bme280ServiceEvent::TemperatureOffsetWrite(value) => {
                let mut data = FLASH_MANAGER.get().get_last_calibration_data().await;
                data.bme_temperature = f32::from_le_bytes(value);
                data
            }
            Bme280ServiceEvent::PressureOffsetWrite(value) => {
                let mut data = FLASH_MANAGER.get().get_last_calibration_data().await;
                data.bme_pressure = f32::from_le_bytes(value);
                data
            }
            _ => return
        };

        next_calibration_data.version += 1;

        let _ = FLASH_MANAGER.get().write_calibration_data(&next_calibration_data).await;
    }
}

impl_settings_event_consumer!(
    DiNotificationSettings,
    DeviceInformationServiceEvent,
    BatteryVoltage,
    Temperature,
    Debug
);

impl_is_task_enabled!(BmeNotificationSettings, humidity, pressure, temperature);
impl_is_task_enabled!(DiNotificationSettings, debug, battery_voltage, temperature);
impl_is_task_enabled!(
    AdcNotificationSettings,
    voltage0,
    voltage1,
    voltage2,
    voltage3,
    voltage4,
    voltage5,
    voltage6,
    elapsed,
    samples
);
impl_is_task_enabled!(ColorNotificationSettings, red, green, blue, white);
impl_is_task_enabled!(AccelerometerNotificationSettings, x, y, z);

impl_timeout_event_characteristic!(AdcServiceEvent);
impl_timeout_event_characteristic!(Bme280ServiceEvent);
impl_timeout_event_characteristic!(DeviceInformationServiceEvent);
impl_timeout_event_characteristic!(ColorServiceEvent);
impl_timeout_event_characteristic!(AccelerometerServiceEvent);

impl_read_event_channel!("adc", ADC_SERVICE_EVENTS, ADC_EVENT_PROCESSOR);
impl_read_event_channel!("bme", BME_SERVICE_EVENTS, BME_EVENT_PROCESSOR);
impl_read_event_channel!("di", DI_SERVICE_EVENTS, DEVICE_EVENT_PROCESSOR);
impl_read_event_channel!("color", COLOR_SERVICE_EVENTS, COLOR_EVENT_PROCESSOR);
impl_read_event_channel!(
    "accelerometer",
    ACCELEROMETER_SERVICE_EVENTS,
    ACCELEROMETER_EVENT_PROCESSOR
);
