#![macro_use]
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]

extern crate alloc;

use defmt::{info, unwrap};
#[allow(unused)]
use defmt_rtt as _;
use embassy_executor::Spawner;
#[allow(unused)]
use embassy_nrf as _;
use embedded_alloc::Heap;
use nrf_softdevice::ble::{gatt_server, peripheral, Connection};
use nrf_softdevice::Softdevice;
#[allow(unused)]
use panic_probe as _;
use rclite::Arc;

use crate::common::ble::event_processor::{
    read_accelerometer_notification_settings_channel, read_adc_notification_settings_channel,
    read_bme_notification_settings_channel, read_color_notification_settings_channel,
    read_di_notification_settings_channel,
};
use crate::common::ble::services::{BleServer, BleServerEvent};
use crate::common::ble::softdevice::{prepare_adv_scan_data, prepare_softdevice_config};
use crate::common::ble::{
    ACCELEROMETER_EVENT_PROCESSOR, ACCELEROMETER_SERVICE_EVENTS, ADC_EVENT_PROCESSOR,
    ADC_SERVICE_EVENTS, BME_EVENT_PROCESSOR, BME_SERVICE_EVENTS, COLOR_EVENT_PROCESSOR,
    COLOR_SERVICE_EVENTS, DEVICE_EVENT_PROCESSOR, DI_SERVICE_EVENTS, SERVER,
};
use crate::common::device::device_manager::DeviceManager;
use crate::common::device::nrf_temp::notify_nrf_temp;
use crate::common::device::task::adc::{read_saadc_battery_voltage_task, read_saadc_task};
use crate::common::device::task::i2c::read_i2c0_task;
use crate::common::device::task::spi::epd_task;
use common::util::ble_debugger::ble_debug_notify_task;
use crate::common::device::ui::UI_STORE;

#[path = "../common.rs"]
mod common;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    let device_manager = DeviceManager::new(spawner).await.unwrap();
    let sd_config = prepare_softdevice_config();
    let sd = Softdevice::enable(&sd_config);
    let server = unwrap!(BleServer::new(sd));

    SERVER.init_ro(server);

    unwrap!(spawner.spawn(epd_task(
        Arc::clone(&device_manager.spi2_pins),
        Arc::clone(&device_manager.epd_control_pins)
    )));

    unwrap!(spawner.spawn(read_saadc_battery_voltage_task(Arc::clone(&device_manager.saadc_pins))));
    unwrap!(spawner.spawn(read_saadc_task(Arc::clone(&device_manager.saadc_pins))));
    unwrap!(spawner.spawn(read_i2c0_task(Arc::clone(&device_manager.bbi2c0_pins))));
    unwrap!(spawner.spawn(ble_debug_notify_task()));
    unwrap!(spawner.spawn(softdevice_task(sd)));
    unwrap!(spawner.spawn(notify_nrf_temp(sd)));

    unwrap!(spawner.spawn(read_adc_notification_settings_channel()));
    unwrap!(spawner.spawn(read_bme_notification_settings_channel()));
    unwrap!(spawner.spawn(read_di_notification_settings_channel()));
    unwrap!(spawner.spawn(read_accelerometer_notification_settings_channel()));
    unwrap!(spawner.spawn(read_color_notification_settings_channel()));

    let (adv_data, scan_data) = prepare_adv_scan_data();

    info!("Init has finished successfully");

    loop {
        let config = peripheral::Config::default();
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected { adv_data, scan_data };
        info!("Waiting for connection");
        let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &config).await);
        unwrap!(spawner.spawn(handle_connection(conn)));
        UI_STORE.lock().await.num_connections = Connection::iter().count() as u8
    }
}

#[embassy_executor::task(pool_size = 3)]
async fn handle_connection(connection: Connection) {
    DEVICE_EVENT_PROCESSOR.register_connection(&connection).await;
    BME_EVENT_PROCESSOR.register_connection(&connection).await;
    ADC_EVENT_PROCESSOR.register_connection(&connection).await;
    ACCELEROMETER_EVENT_PROCESSOR.register_connection(&connection).await;
    COLOR_EVENT_PROCESSOR.register_connection(&connection).await;

    let server_fut = gatt_server::run(&connection, SERVER.get(), |e| match e {
        BleServerEvent::Dis(event) => {
            if DI_SERVICE_EVENTS.try_send((connection.clone(), event)).is_err() {
                ble_debug!("Failed to send DI service event")
            }
        }
        BleServerEvent::Adc(event) => {
            if ADC_SERVICE_EVENTS.try_send((connection.clone(), event)).is_err() {
                ble_debug!("Failed to send DI service event")
            }
        }
        BleServerEvent::Bme280(event) => {
            if BME_SERVICE_EVENTS.try_send((connection.clone(), event)).is_err() {
                ble_debug!("Failed to send DI service event")
            }
        }
        BleServerEvent::Accelerometer(event) => {
            if ACCELEROMETER_SERVICE_EVENTS.try_send((connection.clone(), event)).is_err() {
                ble_debug!("Failed to send Accelerometer service event")
            }
        }
        BleServerEvent::Color(event) => {
            if COLOR_SERVICE_EVENTS.try_send((connection.clone(), event)).is_err() {
                ble_debug!("Failed to send Color service event")
            }
        }
    });

    let _error = server_fut.await;
    DEVICE_EVENT_PROCESSOR.drop_connection(&connection).await;
    BME_EVENT_PROCESSOR.drop_connection(&connection).await;
    ADC_EVENT_PROCESSOR.drop_connection(&connection).await;
    ACCELEROMETER_EVENT_PROCESSOR.drop_connection(&connection).await;
    COLOR_EVENT_PROCESSOR.drop_connection(&connection).await;
    info!("Connection closed");
}
