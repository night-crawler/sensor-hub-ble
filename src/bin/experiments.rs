#![macro_use]
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::mem;

use defmt::{*};
#[allow(unused)]
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, peripherals, Peripherals};
#[allow(unused)]
use embassy_nrf as _;
use embassy_nrf::config::{HfclkSource, LfclkSource};
use embassy_nrf::gpio::{AnyPin, Level, Output, OutputDrive, Pin};
use embassy_nrf::interrupt::Priority;
use embassy_nrf::twim::{self, Twim};
use embassy_time::{Duration, Timer};
use nrf_softdevice::{raw, Softdevice};
#[allow(unused)]
use panic_probe as _;

bind_interrupts!(pub(crate) struct Irqs {
    // SAADC => saadc::InterruptHandler;
    // TEMP => temp::InterruptHandler;
    SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1 => twim::InterruptHandler<peripherals::TWISPI1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let board = prepare_nrf_peripherals();
    let mut blue_led = Output::new(board.P0_06.degrade(), Level::High, OutputDrive::Standard);

    blink(&mut blue_led).await;

    let config = prepare_softdevice_config();
    blink(&mut blue_led).await;

    let sd = Softdevice::enable(&config);
    blink(&mut blue_led).await;

    let mut twi_config = twim::Config::default();
    let mut twi = Twim::new(board.TWISPI1, Irqs, board.P1_11, board.P1_12, twi_config);

    blink(&mut blue_led).await;
}


async fn blink(pin: &mut Output<'static, AnyPin>) {
    pin.set_low();
    Timer::after(Duration::from_millis(250)).await;
    pin.set_high();
    Timer::after(Duration::from_millis(250)).await;
}


fn prepare_nrf_peripherals() -> Peripherals {
    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = HfclkSource::ExternalXtal;
    config.lfclk_source = LfclkSource::ExternalXtal;
    config.gpiote_interrupt_priority = Priority::P2;
    config.time_interrupt_priority = Priority::P2;
    embassy_nrf::init(config)
}


pub(crate) fn prepare_softdevice_config() -> nrf_softdevice::Config {
    nrf_softdevice::Config {
        clock: Some(raw::nrf_clock_lf_cfg_t {
            source: raw::NRF_CLOCK_LF_SRC_RC as u8,
            rc_ctiv: 1,
            rc_temp_ctiv: 2,
            accuracy: raw::NRF_CLOCK_LF_ACCURACY_20_PPM as u8,
        }),
        conn_gap: Some(raw::ble_gap_conn_cfg_t {
            conn_count: 3,
            event_length: 24,
        }),
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t { attr_tab_size: 32768 }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1,
            periph_role_count: 3,
            central_role_count: 0,
            central_sec_count: 0,
            _bitfield_1: raw::ble_gap_cfg_role_count_t::new_bitfield_1(0),
        }),
        gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
            p_value: b"Sensor Hub BLE" as *const u8 as _,
            current_len: 14,
            max_len: 14,
            write_perm: unsafe { mem::zeroed() },
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(raw::BLE_GATTS_VLOC_STACK as u8),
        }),
        ..Default::default()
    }
}

pub(crate) fn prepare_adv_scan_data() -> (&'static [u8], &'static [u8]) {
    static ADV_DATA: [u8; 23] = [
        0x02, 0x01, raw::BLE_GAP_ADV_FLAGS_LE_ONLY_GENERAL_DISC_MODE as u8,
        0x03, 0x03, 0x09, 0x18,
        0x0F, 0x09, b'S', b'e', b'n', b's', b'o', b'r', b' ', b'H', b'u', b'b', b' ', b'B', b'L', b'E'
    ];
    // scan_rsp_data
    static SCAN_DATA: [u8; 4] = [
        0x03, 0x03, 0x09, 0x18,
    ];

    (&ADV_DATA, &SCAN_DATA)
}
