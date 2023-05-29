use core::mem;

use nrf_softdevice::{Config, raw};

use crate::common::device::config::NUM_CONNECTIONS;

pub(crate) fn prepare_softdevice_config() -> Config {
    Config {
        clock: Some(raw::nrf_clock_lf_cfg_t {
            source: raw::NRF_CLOCK_LF_SRC_RC as u8,
            rc_ctiv: 1,
            rc_temp_ctiv: 2,
            accuracy: raw::NRF_CLOCK_LF_ACCURACY_20_PPM as u8,
        }),
        conn_gap: Some(raw::ble_gap_conn_cfg_t {
            conn_count: NUM_CONNECTIONS as u8,
            event_length: 24,
        }),
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t { attr_tab_size: 32768 }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1,
            periph_role_count: NUM_CONNECTIONS as u8,
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
