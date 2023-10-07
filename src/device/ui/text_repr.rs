use alloc::format;
use alloc::string::{String, ToString};
use crate::common::device::ui::ui_store::UiStore;
pub(crate) struct TextRepr {
    pub(crate) bat: String,
    pub(crate) nrf_voltages: String,
    pub(crate) adc_voltages: String,
    pub(crate) temp: String,
    pub(crate) humidity: String,
    pub(crate) pressure: String,
    pub(crate) lux_text: String,
    pub(crate) cct_text: String,
    pub(crate) rgbw_text: String,
    pub(crate) xyz_text: String,
    pub(crate) connections: String,
}

impl TextRepr {
    /// Depending on battery voltage, return a text representation that matches to a battery icon.
    /// "0" - 5 values used to represent battery level.
    /// "0" - 3.2V
    /// "5" -  4.2V
    /// "6" - charging
    fn get_charge_level_icon_text(v_bat: f32) -> &'static str {
        let max_voltage = 4.2f32;
        let min_voltage = 3.2f32;
        let v_bat = v_bat.min(max_voltage).max(min_voltage);
        let v_bat = (v_bat - min_voltage) / (max_voltage - min_voltage);
        let v_bat = v_bat * 5.0;
        let v_bat = v_bat as u8;
        match v_bat {
            0 => "0",
            1 => "1",
            2 => "2",
            3 => "3",
            4 => "4",
            5 => "5",
            _ => "6",
        }
    }

    
}

impl From<&UiStore> for TextRepr {
    fn from(value: &UiStore) -> Self {
        let bat_text = Self::get_charge_level_icon_text(value.bat_voltage);
        Self {
            bat: bat_text.to_string(),
            nrf_voltages: value.nrf_adc_voltages[..7].iter().map(|v| format!("{:.2}", v)).collect::<String>(),
            adc_voltages: value.adc_voltages.iter().map(|v| format!("{:.2}", v)).collect::<String>(),
            temp: format!("{:.1}", value.temperature),
            humidity: format!("{:.1}%", value.humidity),
            pressure: format!("{:.1}", value.pressure / 100.0),
            lux_text: format!("{:.1}", value.lux as u32),
            cct_text: format!("{:.1}", value.cct as u32),
            rgbw_text: format!("R:{} G:{} B:{} W:{}; BAT:{:.2}", value.r, value.g, value.b, value.w, value.bat_voltage),
            xyz_text: format!("X: {:.2} Y: {:.2} Z: {:.2}", value.x, value.y, value.z),
            connections: format!("{}", value.num_connections),
        }
    }
}
