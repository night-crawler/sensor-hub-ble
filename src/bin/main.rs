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
use futures::{FutureExt, pin_mut};
use futures::select_biased;
use nrf_softdevice::ble::{gatt_server, peripheral};
use nrf_softdevice::Softdevice;
#[allow(unused)]
use panic_probe as _;

use crate::common::ble::services::{AdcServiceEvent, BleServer, BleServerEvent, Bme280ServiceEvent, DeviceInformationServiceEvent};
use crate::common::ble::softdevice::{prepare_adv_scan_data, prepare_softdevice_config};
use crate::common::device::adc::ADC_TIMEOUT;
use crate::common::device::ble_debugger::ble_debug_notify_task;
use crate::common::device::device_manager::DeviceManager;
use crate::common::device::led_animation::{LED, LedState, LedStateAnimation};
use crate::common::device::nrf_temp::notify_nrf_temp;

#[path = "../common.rs"]
mod common;


#[global_allocator]
static HEAP: Heap = Heap::empty();


#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}


// pub(crate) static CONNECTIONS =


#[embassy_executor::main]
async fn main(spawner: Spawner) {
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 128;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    let mut device_manager = DeviceManager::new(spawner).await.unwrap();
    LED.lock().await.blink_short(LedState::White).await;

    let sd_config = prepare_softdevice_config();
    LED.lock().await.blink_short(LedState::White).await;

    let sd = Softdevice::enable(&sd_config);
    LED.lock().await.blink_short(LedState::White).await;

    let server = unwrap!(BleServer::new(sd));
    LED.lock().await.blink_short(LedState::White).await;

    unwrap!(spawner.spawn(softdevice_task(sd)));

    let (adv_data, scan_data) = prepare_adv_scan_data();

    info!("Init has finished successfully");


    loop {
        let config = peripheral::Config::default();
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected { adv_data, scan_data };
        let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &config).await);

        LedStateAnimation::blink_long(&[LedState::Purple]);
        Timer::after(Duration::from_millis(1000)).await;

        let _ = server.dis.battery_level_set(&50);

        // let twim0: &mut I2CPins<TWISPI0> = &mut device_manager.i2c0;

        let ble_debug_fut = ble_debug_notify_task(&server, &conn);
        let temp_fut = notify_nrf_temp(sd, &server, &conn);
        // let adc_fut = notify_adc_value(&mut device_manager.saadc, &server, &conn);
        // let i2c0_fut = read_i2c0(twim0, &server, &conn);

        let server_fut = gatt_server::run(&conn, &server, |e| match e {
            BleServerEvent::Dis(event) => match event {
                DeviceInformationServiceEvent::BatteryLevelCccdWrite { notifications } => {
                    let state = if notifications { LedState::Green } else { LedState::Red };
                    LedStateAnimation::blink_short(&[state]);
                }
                DeviceInformationServiceEvent::TempCccdWrite { notifications } => {
                    let state = if notifications { LedState::Green } else { LedState::Red };
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
            }
        });

        pin_mut!(
            // adc_fut,
            server_fut,
            temp_fut,
            // i2c0_fut
        );

        let return_state = select_biased! {
            _ = server_fut.fuse() => {
                &[LedState::Purple, LedState::Yellow, LedState::White]
            }
            // _ = adc_fut.fuse() => {
            //     &[LedState::Red, LedState::Green, LedState::Blue]
            // }
            _ = temp_fut.fuse() => {
                &[LedState::White, LedState::Cyan, LedState::Purple]
            }
            // _ = i2c0_fut.fuse() => {
            //     &[LedState::Green, LedState::Red, LedState::Yellow]
            // }
            _ = ble_debug_fut.fuse() => {
                &[LedState::Cyan, LedState::Red, LedState::Yellow]
            }
        };

        LedStateAnimation::sweep_long(return_state);
    }
}


