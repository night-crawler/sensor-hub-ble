#![macro_use]
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]

extern crate alloc;

use core::sync::atomic::Ordering;

use defmt::{info, unwrap};
#[allow(unused)]
use defmt_rtt as _;
use embassy_executor::Spawner;
#[allow(unused)]
use embassy_nrf as _;
use embassy_time::{Duration, Timer};
use embedded_alloc::Heap;
use nrf_softdevice::ble::{gatt_server, peripheral};
use nrf_softdevice::Softdevice;
#[allow(unused)]
use panic_probe as _;
use rclite::Arc;

use crate::common::ble::services::{
    AdcServiceEvent, BleServer, BleServerEvent, Bme280ServiceEvent, DeviceInformationServiceEvent,
};
use crate::common::ble::softdevice::{prepare_adv_scan_data, prepare_softdevice_config};
use crate::common::ble::SERVER;
use crate::common::device::adc::ADC_TIMEOUT;
use crate::common::device::ble_debugger::ble_debug_notify_task;
use crate::common::device::device_manager::DeviceManager;
use crate::common::device::i2c::read_i2c0_task;
use crate::common::device::led_animation::{LedState, LedStateAnimation, LED};
use crate::common::device::spi::epd_task;

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
    LED.lock().await.blink_short(LedState::White).await;

    let sd_config = prepare_softdevice_config();
    LED.lock().await.blink_short(LedState::White).await;

    let sd = Softdevice::enable(&sd_config);
    LED.lock().await.blink_short(LedState::White).await;

    let server = unwrap!(BleServer::new(sd));
    LED.lock().await.blink_short(LedState::White).await;

    SERVER.init_ro(server);

    unwrap!(spawner.spawn(epd_task(
        Arc::clone(&device_manager.spi2),
        Arc::clone(&device_manager.epd_control_pins)
    )));

    unwrap!(spawner.spawn(read_i2c0_task(Arc::clone(&device_manager.bbi2c0))));
    unwrap!(spawner.spawn(ble_debug_notify_task()));
    unwrap!(spawner.spawn(softdevice_task(sd)));

    let (adv_data, scan_data) = prepare_adv_scan_data();

    info!("Init has finished successfully");

    loop {
        let config = peripheral::Config::default();
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
            adv_data,
            scan_data,
        };
        info!("Waiting for connection");
        let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &config).await);

        LedStateAnimation::blink_long(&[LedState::Purple]);
        Timer::after(Duration::from_millis(1000)).await;

        let server_fut = gatt_server::run(&conn, SERVER.get(), |e| match e {
            BleServerEvent::Dis(event) => match event {
                DeviceInformationServiceEvent::BatteryLevelCccdWrite { notifications } => {
                    let state = if notifications {
                        LedState::Green
                    } else {
                        LedState::Red
                    };
                    LedStateAnimation::blink_short(&[state]);
                }
                DeviceInformationServiceEvent::TempCccdWrite { notifications } => {
                    let state = if notifications {
                        LedState::Green
                    } else {
                        LedState::Red
                    };
                    LedStateAnimation::blink_short(&[state]);
                }
                DeviceInformationServiceEvent::DebugCccdWrite { .. } => {}
            },
            BleServerEvent::Adc(event) => match event {
                AdcServiceEvent::Voltage0CccdWrite { .. } => {}
                AdcServiceEvent::Voltage1CccdWrite { .. } => {}
                AdcServiceEvent::Voltage2CccdWrite { .. } => {}
                AdcServiceEvent::Voltage3CccdWrite { .. } => {}
                AdcServiceEvent::Voltage4CccdWrite { .. } => {}
                AdcServiceEvent::Voltage5CccdWrite { .. } => {}
                AdcServiceEvent::SamplesCccdWrite { .. } => {}
                AdcServiceEvent::ElapsedCccdWrite { .. } => {}
                AdcServiceEvent::TimeoutCccdWrite { .. } => {}
                AdcServiceEvent::TimeoutWrite(timeout) => {
                    ADC_TIMEOUT.store(timeout, Ordering::SeqCst);
                }
            },
            BleServerEvent::Bme280(event) => match event {
                Bme280ServiceEvent::TempCccdWrite { .. } => {}
                Bme280ServiceEvent::HumidityCccdWrite { .. } => {}
                Bme280ServiceEvent::PressureCccdWrite { .. } => {}
                Bme280ServiceEvent::TimeoutWrite(_) => {}
                Bme280ServiceEvent::TimeoutCccdWrite { .. } => {}
            },
        });

        LedStateAnimation::sweep_long(&[LedState::White]);
        server_fut.await;
    }
}
