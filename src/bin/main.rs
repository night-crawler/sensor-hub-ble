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
use embassy_time::{Duration, Timer};
use embedded_alloc::Heap;
use futures::future::{Either, select};
use futures::pin_mut;
use nrf_softdevice::{Softdevice, temperature_celsius};
use nrf_softdevice::ble::{Connection, gatt_server, peripheral};
#[allow(unused)]
use panic_probe as _;

use crate::common::ble::conv::ConvExt;
use crate::common::ble::services::{AdcServiceEvent, BleServer, BleServerEvent, DeviceInformationServiceEvent};
use crate::common::ble::softdevice::{prepare_adv_scan_data, prepare_softdevice_config};
use crate::common::device::adc::{ADC_TIMEOUT, notify_adc_value};
use crate::common::device::device_manager::DeviceManager;
use crate::common::device::led_animation::{LED, LedState, LedStateAnimation};

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
    LED.lock().await.blink_short(LedState::White).await;

    let sd_config = prepare_softdevice_config();
    LED.lock().await.blink_short(LedState::White).await;

    let sd = Softdevice::enable(&sd_config);
    LED.lock().await.blink_short(LedState::White).await;

    let server = unwrap!(BleServer::new(sd));
    LED.lock().await.blink_short(LedState::White).await;

    unwrap!(spawner.spawn(softdevice_task(sd)));

    let (adv_data, scan_data) = prepare_adv_scan_data();

    loop {
        let config = peripheral::Config::default();
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected { adv_data, scan_data };
        let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &config).await);

        LedStateAnimation::blink_long(&[LedState::Purple]);
        Timer::after(Duration::from_millis(1000)).await;

        // let init_temp = device_manager.temp.read().await.to_num::<f32>().as_temp();
        let _ = server.dis.battery_level_set(&50);
        // let _ = server.dis.temp_set(&init_temp);
        let _ = server.dis.debug_set(b"Sample\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");

        let temp_fut = notify_nrf_temp_value(sd, &server, &conn);
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

        pin_mut!(adc_fut, server_fut, temp_fut);


        let adc_ser = select(adc_fut, server_fut);
        pin_mut!(adc_ser);

        match select(adc_ser, temp_fut).await {
            Either::Left((adc_ser, _)) => {
                match adc_ser {
                    Either::Left((_, _)) => {
                        LedStateAnimation::sweep_long(&[LedState::Red, LedState::Green]);
                    }
                    Either::Right((_, _)) => {
                        LedStateAnimation::sweep_long(&[LedState::Purple, LedState::Cyan]);
                    }
                }
            }
            Either::Right((_, _)) => {
                LedStateAnimation::sweep_long(&[LedState::Green, LedState::Yellow]);
            }
        }

        // match select(adc_fut, server_fut).await {
        //     Either::Left((_, _)) => {
        //         LedStateAnimation::sweep_long(&[LedState::Red, LedState::Green]);
        //     }
        //     Either::Right((_, _)) => {
        //         LedStateAnimation::sweep_long(&[LedState::Purple, LedState::Cyan]);
        //     }
        // };
    }
}

async fn notify_nrf_temp_value<'a>(sd: &Softdevice, server: &'a BleServer, connection: &'a Connection) {
    loop {
        let value = match temperature_celsius(sd) {
            Ok(value) => {
                value.to_num::<f32>().as_temp()
            }
            Err(_) => {
                LedStateAnimation::blink_long(&[LedState::Red]);
                continue;
            }
        };
        match server.dis.temp_notify(connection, &value) {
            Ok(_) => {}
            Err(_) => {
                let _ = server.dis.temp_set(&value);
            }
        }
        Timer::after(Duration::from_millis(ADC_TIMEOUT.load(Ordering::Relaxed) as u64)).await;
    }
}
