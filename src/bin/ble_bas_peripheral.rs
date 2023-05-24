#![macro_use]
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

use defmt::{*, info};
#[allow(unused)]
use defmt_rtt as _;
use embassy_executor::Spawner;
#[allow(unused)]
use embassy_nrf as _;
use embassy_nrf::interrupt::Interrupt;
use embassy_nrf::saadc::{CallbackResult, Saadc};
use embassy_nrf::timer::Frequency;
use embassy_time::{Duration, Timer};
use embedded_alloc::Heap;
use futures::future::{Either, select};
use futures::pin_mut;
use nrf_softdevice::ble::{Connection, gatt_server, peripheral};
use nrf_softdevice::Softdevice;
#[allow(unused)]
use panic_probe as _;

use crate::common::ble::services::{AdcServiceEvent, BleServer, BleServerEvent, DeviceInformationServiceEvent};
use crate::common::ble::softdevice::{prepare_adv_scan_data, prepare_softdevice_config};
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
        let _ = server.dis.temp_notify(&conn, &-32);
        let _ = server.dis.debug_notify(&conn, b"Sample\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");

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
                AdcServiceEvent::Voltage1CccdWrite {
                    indications,
                    notifications,
                } => {
                    info!("foo indications: {}, notifications: {}", indications, notifications)
                }
            },
        });

        pin_mut!(adc_fut);
        pin_mut!(server_fut);

        match select(adc_fut, server_fut).await {
            Either::Left((_, _)) => {
                LedStateAnimation::sweep_long(&[LedState::Red, LedState::White]);
            }
            Either::Right((_, _)) => {
                LedStateAnimation::sweep_long(&[LedState::Purple, LedState::Cyan]);
            }
        };
    }
}


async fn notify_adc_value<'a>(saadc: &'a mut Saadc<'_, 1>, server: &'a BleServer, connection: &'a Connection) {
    loop {
        // let mut buf = [0i16; 1];
        // saadc.sample(&mut buf).await;
        //
        // // We only sampled one ADC channel.
        // let adc_raw_value = buf[0].unsigned_abs();
        //
        // // Try and notify the connected client of the new ADC value.
        // match server.adc.voltage1_notify(connection, &(adc_raw_value as i32)) {
        //     Ok(_) => {},
        //     Err(_) => unwrap!(server.adc.voltage1_set(&(adc_raw_value as i32))),
        // };
        //
        // // Sleep for one second.
        // Timer::after(Duration::from_secs(1)).await

        let mut bufs = [[[0; 1]; 500]; 2];

        let mut c = 0;
        let mut accum: u64 = 0;

        let mut t0 = unsafe { embassy_nrf::peripherals::TIMER2::steal() };
        let mut ppi0 = unsafe { embassy_nrf::peripherals::PPI_CH10::steal() };
        let mut ppi1 = unsafe { embassy_nrf::peripherals::PPI_CH11::steal() };

        saadc
            .run_task_sampler(
                &mut t0,
                &mut ppi0,
                &mut ppi1,
                Frequency::F16MHz,
                1000, // We want to sample at 1KHz
                &mut bufs,
                move |buf| {
                    for b in buf {
                        accum += b[0] as u64;
                    }
                    c += buf.len();

                    if c > 1000 {
                        accum /= c as u64;
                        let _ = server.adc.voltage1_notify(connection, &(accum as i32));
                        c = 0;
                        accum = 0;
                        return CallbackResult::Stop;
                    }
                    CallbackResult::Continue
                },
            )
            .await;
    }
}