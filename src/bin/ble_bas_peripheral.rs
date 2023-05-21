#![macro_use]
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::mem;

use defmt::{*, info};
#[allow(unused)]
use defmt_rtt as _;
use embassy_executor::Spawner;
#[allow(unused)]
use embassy_nrf as _;
use embassy_nrf::Peripheral;
use embassy_nrf::gpio::Pin;
use embassy_time::{Duration, Timer};
use embedded_alloc::Heap;
use futures::future::join3;
use nrf_softdevice::{raw, Softdevice};
use nrf_softdevice::ble::{gatt_server, peripheral};
#[allow(unused)]
use panic_probe as _;
// use smallvec::SmallVec;
use static_cell::StaticCell;

use crate::common::pins::DeviceManager;

#[path = "../common.rs"]
mod common;


#[global_allocator]
static HEAP: Heap = Heap::empty();


#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}


static PIN_MANAGER: StaticCell<DeviceManager> = StaticCell::new();


#[embassy_executor::task]
async fn blink_task(pw: &'static mut DeviceManager) -> ! {
    pw.pin_group1.all_high();
    pw.pin_group2.all_high();
    pw.pin_group3.all_high();
    Timer::after(Duration::from_millis(5)).await;

    loop {
        let f1 = pw.pin_group1.sweep(1);
        let f2 = pw.pin_group2.sweep(2);
        let f3 = pw.pin_group3.sweep(3);

        join3(f1, f2, f3).await;
    }
}

#[nrf_softdevice::gatt_service(uuid = "180f")]
struct BatteryService {
    #[characteristic(uuid = "2a19", read, notify)]
    battery_level: u8,
}

#[nrf_softdevice::gatt_service(uuid = "9e7312e0-2354-11eb-9f10-fbc30a62cf38")]
struct FooService {
    #[characteristic(uuid = "9e7312e0-2354-11eb-9f10-fbc30a63cf38", read, write, notify, indicate)]
    foo: u16,
}

#[nrf_softdevice::gatt_server]
struct Server {
    bas: BatteryService,
    foo: FooService,
}


#[embassy_executor::main]
async fn main(spawner: Spawner) {
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    info!("Hello World!");

    let pm = PIN_MANAGER.init(DeviceManager::new());

    match spawner.spawn(blink_task(pm)) {
        Ok(_) => {
            info!("Spawned")
        }
        Err(err) => {
            info!("Failed to spawn: {}", err)
        }
    };

    let config = nrf_softdevice::Config {
        clock: Some(raw::nrf_clock_lf_cfg_t {
            source: raw::NRF_CLOCK_LF_SRC_RC as u8,
            rc_ctiv: 1,
            rc_temp_ctiv: 2,
            accuracy: raw::NRF_CLOCK_LF_ACCURACY_20_PPM as u8,
        }),
        conn_gap: Some(raw::ble_gap_conn_cfg_t {
            conn_count: 6,
            event_length: 24,
        }),
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t { attr_tab_size: 32768 }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1,
            periph_role_count: 3,
            central_role_count: 3,
            central_sec_count: 0,
            _bitfield_1: raw::ble_gap_cfg_role_count_t::new_bitfield_1(0),
        }),
        gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
            p_value: b"HelloRust" as *const u8 as _,
            current_len: 9,
            max_len: 9,
            write_perm: unsafe { mem::zeroed() },
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(raw::BLE_GATTS_VLOC_STACK as u8),
        }),
        ..Default::default()
    };

    let sd = Softdevice::enable(&config);
    let server = unwrap!(Server::new(sd));
    unwrap!(spawner.spawn(softdevice_task(sd)));

    #[rustfmt::skip]
        let adv_data = &[
        0x02, 0x01, raw::BLE_GAP_ADV_FLAGS_LE_ONLY_GENERAL_DISC_MODE as u8,
        0x03, 0x03, 0x09, 0x18,
        0x0a, 0x09, b'H', b'e', b'l', b'l', b'o', b'R', b'u', b's', b't',
    ];
    #[rustfmt::skip]
        let scan_data = &[
        0x03, 0x03, 0x09, 0x18,
    ];

    loop {
        let config = peripheral::Config::default();
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected { adv_data, scan_data };
        let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &config).await);

        info!("advertising done!");

        // Run the GATT server on the connection. This returns when the connection gets disconnected.
        //
        // Event enums (ServerEvent's) are generated by nrf_softdevice::gatt_server
        // proc macro when applied to the Server struct above
        let e = gatt_server::run(&conn, &server, |e| match e {
            ServerEvent::Bas(e) => match e {
                BatteryServiceEvent::BatteryLevelCccdWrite { notifications } => {
                    info!("battery notifications: {}", notifications)
                }
            },
            ServerEvent::Foo(e) => match e {
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
        })
            .await;

        info!("gatt_server run exited with error: {:?}", e);
    }
}

#[cfg(test)]
mod test {}