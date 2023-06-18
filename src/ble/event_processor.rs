use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, Ordering};

use defmt::info;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Duration;
use nrf_softdevice::ble::Connection;

use crate::common::ble::{
    ADC_SERVICE_EVENTS, BME_SERVICE_EVENTS, BME_TASK_CONDITION, DI_SERVICE_EVENTS,
    NOTIFICATION_SETTINGS,
};
use crate::common::ble::services::{
    AdcServiceEvent, Bme280ServiceEvent, DeviceInformationServiceEvent,
};
use crate::common::ble::traits::DetermineTaskState;
use crate::common::util::condition::Condition;
use crate::impl_set_notification;

#[derive(Default)]
pub(crate) struct AdcNotificationSettings {
    pub(crate) voltage1: bool,
    pub(crate) voltage2: bool,
    pub(crate) voltage3: bool,
    pub(crate) voltage4: bool,
    pub(crate) voltage5: bool,
    pub(crate) voltage6: bool,
    pub(crate) voltage7: bool,
    pub(crate) voltage8: bool,
    pub(crate) samples: bool,
    pub(crate) elapsed: bool,
}

#[derive(Default)]
pub(crate) struct BmeNotificationSettings {
    pub(crate) temperature: bool,
    pub(crate) humidity: bool,
    pub(crate) pressure: bool,
}

#[derive(Default)]
pub(crate) struct DiNotificationSettings {
    pub(crate) temperature: bool,
    pub(crate) battery_level: bool,
    pub(crate) debug: bool,
}

impl DetermineTaskState for BmeNotificationSettings {
    fn determine_task_state(&self) -> bool {
        self.humidity || self.pressure || self.temperature
    }
}

pub(crate) struct EventProcessor {
    adc_settings: Mutex<ThreadModeRawMutex, BTreeMap<Connection, AdcNotificationSettings>>,
    adc_timeout: AtomicU32,

    bme_settings: Mutex<ThreadModeRawMutex, BTreeMap<Connection, BmeNotificationSettings>>,
    bme_timeout: AtomicU32,

    di_settings: Mutex<ThreadModeRawMutex, BTreeMap<Connection, DiNotificationSettings>>,
    di_timeout: AtomicU32,
}

impl Default for EventProcessor {
    fn default() -> Self {
        EventProcessor {
            adc_settings: Mutex::new(Default::default()),
            adc_timeout: AtomicU32::new(1000),

            bme_settings: Mutex::new(Default::default()),
            bme_timeout: AtomicU32::new(1000),

            di_settings: Mutex::new(Default::default()),
            di_timeout: AtomicU32::new(1000),
        }
    }
}

impl EventProcessor {
    pub(crate) async fn process_adc_event(&self, connection: Connection, event: AdcServiceEvent) {
        let mut settings_map = self.adc_settings.lock().await;
        let current_settings = settings_map.entry(connection).or_default();

        if let AdcServiceEvent::TimeoutWrite(timeout) = &event {
            self.adc_timeout.store(*timeout, Ordering::SeqCst);
        }

        impl_set_notification!(
            AdcServiceEvent,
            event,
            current_settings,
            Voltage1,
            Voltage2,
            Voltage3,
            Voltage4,
            Voltage5,
            Voltage6,
            Voltage7,
            Voltage8,
            Samples,
            Elapsed
        );
    }

    pub(crate) fn get_adc_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.adc_timeout.load(Ordering::Relaxed) as u64)
    }

    pub(crate) async fn process_bme_event(
        &self,
        connection: Connection,
        event: Bme280ServiceEvent,
    ) {
        let mut settings_map = self.bme_settings.lock().await;
        let current_settings = settings_map.entry(connection).or_default();

        if let Bme280ServiceEvent::TimeoutWrite(timeout) = &event {
            self.bme_timeout.store(*timeout, Ordering::SeqCst);
        }

        impl_set_notification!(
            Bme280ServiceEvent,
            event,
            current_settings,
            Temperature,
            Humidity,
            Pressure
        );

        Self::set_task_enabled_state(&BME_TASK_CONDITION, &settings_map).await;
    }

    async fn set_task_enabled_state<T>(condition: &Condition, settings: &BTreeMap<Connection, T>) where T: DetermineTaskState {
        let should_enable = settings.values().any(|settings| settings.determine_task_state());
        condition.set(should_enable);
    }

    pub(crate) fn get_bme_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.bme_timeout.load(Ordering::Relaxed) as u64)
    }

    pub(crate) async fn process_di_event(
        &self,
        connection: Connection,
        event: DeviceInformationServiceEvent,
    ) {
        let mut settings_map = self.di_settings.lock().await;
        let current_settings = settings_map.entry(connection).or_default();

        if let DeviceInformationServiceEvent::TimeoutWrite(timeout) = &event {
            self.di_timeout.store(*timeout, Ordering::SeqCst);
        }

        impl_set_notification!(
            DeviceInformationServiceEvent,
            event,
            current_settings,
            BatteryLevel,
            Temperature,
            Debug
        );
    }

    pub(crate) fn get_di_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.di_timeout.load(Ordering::Relaxed) as u64)
    }

    pub(crate) async fn drop_connection(&self, connection: &Connection) {
        {
            self.adc_settings.lock().await.remove(connection);
        }
        {
            let mut settings_map = self.bme_settings.lock().await;
            settings_map.remove(connection);
            Self::set_task_enabled_state(&BME_TASK_CONDITION, &settings_map).await;
        }
        {
            self.di_settings.lock().await.remove(connection);
        }
    }
}

#[embassy_executor::task]
pub(crate) async fn read_adc_notification_settings_channel() {
    loop {
        let (connection, settings) = ADC_SERVICE_EVENTS.recv().await;
        NOTIFICATION_SETTINGS.process_adc_event(connection, settings).await;
    }
}

#[embassy_executor::task]
pub(crate) async fn read_bme_notification_settings_channel() {
    loop {
        let (connection, settings) = BME_SERVICE_EVENTS.recv().await;
        NOTIFICATION_SETTINGS.process_bme_event(connection, settings).await;
    }
}

#[embassy_executor::task]
pub(crate) async fn read_di_notification_settings_channel() {
    loop {
        let (connection, settings) = DI_SERVICE_EVENTS.recv().await;
        NOTIFICATION_SETTINGS.process_di_event(connection, settings).await;
    }
}
