#![macro_use]
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

use core::sync::atomic::Ordering;

use defmt::{*};
#[allow(unused)]
use defmt_rtt as _;
use embassy_executor::Spawner;
#[allow(unused)]
use embassy_nrf as _;
use embedded_alloc::Heap;
use futures::future::{Either, select};
use futures::pin_mut;
use nrf_softdevice::ble::{gatt_server, peripheral};
use nrf_softdevice::Softdevice;
#[allow(unused)]
use panic_probe as _;

use crate::common::ble::services::{AdcServiceEvent, BleServer, BleServerEvent, DeviceInformationServiceEvent};
use crate::common::ble::softdevice::{prepare_adv_scan_data, prepare_softdevice_config};
use crate::common::device::adc::{ADC_TIMEOUT, notify_adc_value};
use crate::common::device::device_manager::DeviceManager;
use crate::common::device::led_animation::{LedState, LedStateAnimation};

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
        const HEAP_SIZE: usize = 128;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    let mut device_manager = DeviceManager::new(spawner).await.unwrap();

    let config = prepare_softdevice_config();
    let sd = Softdevice::enable(&config);
    let server = unwrap!(BleServer::new(sd));

    unwrap!(spawner.spawn(softdevice_task(sd)));

    let (adv_data, scan_data) = prepare_adv_scan_data();

    loop {
        let config = peripheral::Config::default();
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected { adv_data, scan_data };
        let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &config).await);

        LedStateAnimation::blink_long(&[LedState::Purple]);

        let _ = server.dis.battery_level_notify(&conn, &50);
        let _ = server.dis.temp_set(&-32);
        let _ = server.dis.debug_set(b"Sample\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");

        let adc_fut = notify_adc_value(&mut device_manager.saadc, &server, &conn);

        let server_fut = gatt_server::run(&conn, &server, |e| match e {
            BleServerEvent::Dis(e) => match e {
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
            BleServerEvent::Adc(e) => match e {
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
        });

        pin_mut!(adc_fut);
        pin_mut!(server_fut);

        match select(adc_fut, server_fut).await {
            Either::Left((_, _)) => {
                LedStateAnimation::sweep_long(&[LedState::Red, LedState::Green]);
            }
            Either::Right((_, _)) => {
                LedStateAnimation::sweep_long(&[LedState::Purple, LedState::Cyan]);
            }
        };
    }
}
