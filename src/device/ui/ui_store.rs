#[derive(Debug, Default)]
pub(crate) struct UiStore {
   pub(crate) nrf_adc_voltages: [f32; 8],
   pub(crate) bat_voltage: f32,
   pub(crate) adc_voltages: [f32; 8],

   pub(crate) r: u16,
   pub(crate) g: u16,
   pub(crate) b: u16,
   pub(crate) w: u16,
   pub(crate) cct: u16,
   pub(crate) lux: f32,

   pub(crate) temperature: f32,
   pub(crate) humidity: f32,
   pub(crate) pressure: f32,

   pub(crate) x: f32,
   pub(crate) y: f32,
   pub(crate) z: f32,

   pub(crate) num_connections: u8,
}
