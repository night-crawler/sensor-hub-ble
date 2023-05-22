#![macro_use]
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::{*, info};
#[allow(unused)]
use defmt_rtt as _;
use embassy_executor::Spawner;
#[allow(unused)]
use embassy_nrf as _;
use embassy_nrf::{bind_interrupts, saadc};
use embedded_alloc::Heap;
use nrf_softdevice::ble::{gatt_server, peripheral};
use nrf_softdevice::Softdevice;
#[allow(unused)]
use panic_probe as _;

use crate::common::ble::services::{BleServer, BleServerEvent, DeviceInformationServiceEvent, FooServiceEvent};
use crate::common::ble::softdevice::{prepare_adv_scan_data, prepare_softdevice_config};
use crate::common::device::device_manager::DeviceManager;

#[path = "../common.rs"]
mod common;


#[global_allocator]
static HEAP: Heap = Heap::empty();


#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}


bind_interrupts!(struct Irqs {
    SAADC => saadc::InterruptHandler;
});


#[embassy_executor::main]
async fn main(spawner: Spawner) {

    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 128;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    let _ = DeviceManager::new(spawner).await.unwrap();

    info!("Hello World!");

    let config = prepare_softdevice_config();
    let sd = Softdevice::enable(&config);
    let server = unwrap!(BleServer::new(sd));
    unwrap!(spawner.spawn(softdevice_task(sd)));

    let (adv_data, scan_data) = prepare_adv_scan_data();


    loop {
        let config = peripheral::Config::default();
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected { adv_data, scan_data };
        let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &config).await);

        info!("advertising done!");

        let _ = server.dis.battery_level_notify(&conn, &50);
        let _ = server.dis.temp_notify(&conn, &0.0);

        let server_fut = gatt_server::run(&conn, &server, |e| match e {
            BleServerEvent::Dis(e) => match e {
                DeviceInformationServiceEvent::BatteryLevelCccdWrite { notifications } => {
                    info!("battery notifications: {}", notifications)
                }
                DeviceInformationServiceEvent::TempCccdWrite { notifications } => {
                    info!("temp notifications: {}", notifications)
                }
            },
            BleServerEvent::Foo(e) => match e {
                FooServiceEvent::FooWrite(val) => {
                    info!("wrote foo: {}", val);
                    if let Err(e) = server.foo.foo_notify(&conn, &(val + 1)) {
                        info!("send notification error: {:?}", e);
                    }
                }
                FooServiceEvent::FooCccdWrite {
                    indications,
                    notifications,
                } => {
                    info!("foo indications: {}, notifications: {}", indications, notifications)
                }
            },
        });

        let result = server_fut.await;

        info!("gatt_server run exited with error: {:?}", result);
    }
}
