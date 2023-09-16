use crate::common::device::config::BLE_DEBUG_ARRAY_LEN;

#[nrf_softdevice::gatt_service(uuid = "180A")]
pub(crate) struct DeviceInformationService {
    #[characteristic(uuid = "00002b18-0000-1000-8999-00805f9b34fb", read, notify)]
    pub(crate) battery_voltage: u16,

    #[characteristic(uuid = "2A6E", read, notify)]
    pub(crate) temperature: i16,

    #[characteristic(uuid = "2BDE", read, notify)]
    pub(crate) debug: [u8; BLE_DEBUG_ARRAY_LEN],

    #[characteristic(uuid = "a0e4d2ba-0002-8000-8789-00805f9b34fb", read, write, notify)]
    pub(crate) timeout: u32,
}

#[nrf_softdevice::gatt_service(uuid = "5c853275-723b-4754-a329-969d8bc8121d")]
pub(crate) struct AdcService {
    #[characteristic(uuid = "00002b18-0000-1000-8000-00805f9b34fb", read, notify)]
    pub(crate) voltage0: u16,

    #[characteristic(uuid = "00002b18-0001-1000-8000-00805f9b34fb", read, notify)]
    pub(crate) voltage1: u16,

    #[characteristic(uuid = "00002b18-0002-1000-8000-00805f9b34fb", read, notify)]
    pub(crate) voltage2: u16,

    #[characteristic(uuid = "00002b18-0003-1000-8000-00805f9b34fb", read, notify)]
    pub(crate) voltage3: u16,

    #[characteristic(uuid = "00002b18-0004-1000-8000-00805f9b34fb", read, notify)]
    pub(crate) voltage4: u16,

    #[characteristic(uuid = "00002b18-0005-1000-8000-00805f9b34fb", read, notify)]
    pub(crate) voltage5: u16,

    #[characteristic(uuid = "00002b18-0006-1000-8000-00805f9b34fb", read, notify)]
    pub(crate) voltage6: u16,

    #[characteristic(uuid = "A0E4D2BA-0000-8000-0000-00805f9b34fb", read, notify)]
    pub(crate) samples: u16,

    #[characteristic(uuid = "A0E4D2BA-0001-8000-0000-00805f9b34fb", read, notify)]
    pub(crate) elapsed: u64,

    #[characteristic(uuid = "a0e4d2ba-0002-8000-0000-00805f9b34fb", read, write, notify)]
    pub(crate) timeout: u32,
}

#[nrf_softdevice::gatt_service(uuid = "5c853275-723b-4754-a329-969d4bc8121e")]
pub(crate) struct Bme280Service {
    #[characteristic(uuid = "2A6E", read, notify)]
    pub(crate) temperature: i16,

    #[characteristic(uuid = "2A6F", read, notify)]
    pub(crate) humidity: u16,

    #[characteristic(uuid = "2A6D", read, notify)]
    pub(crate) pressure: u32,

    #[characteristic(uuid = "a0e4a2ba-0000-8000-0000-00805f9b34fb", read, write, notify)]
    pub(crate) timeout: u32,
}

#[nrf_softdevice::gatt_service(uuid = "5c853275-823b-4754-a329-969d4bc8121e")]
pub(crate) struct AccelerometerService {
    #[characteristic(uuid = "eaeaeaea-0000-0000-0000-00805f9b34fb", read, notify)]
    pub(crate) x: f32,

    #[characteristic(uuid = "eaeaeaea-0000-1000-0000-00805f9b34fb", read, notify)]
    pub(crate) y: f32,

    #[characteristic(uuid = "eaeaeaea-0000-2000-0000-00805f9b34fb", read, notify)]
    pub(crate) z: f32,

    #[characteristic(uuid = "a0e4a2ba-0000-8000-0000-00805f9b34fb", read, write, notify)]
    pub(crate) timeout: u32,
}

#[nrf_softdevice::gatt_service(uuid = "5c853275-923b-4754-a329-969d4bc8121e")]
pub(crate) struct ColorService {
    #[characteristic(uuid = "ebbbbaea-a000-0000-0000-00805f9b34fb", read, notify)]
    pub(crate) red: u16,

    #[characteristic(uuid = "eaeaeaea-b000-1000-0000-00805f9b34fb", read, notify)]
    pub(crate) green: u16,

    #[characteristic(uuid = "eaeaeaea-c000-2000-0000-00805f9b34fb", read, notify)]
    pub(crate) blue: u16,

    #[characteristic(uuid = "eaeaeaea-d000-3000-0000-00805f9b34fb", read, notify)]
    pub(crate) white: u16,

    /// uuid: 0x2AE9
    /// name: Correlated Color Temperature
    /// id: org.bluetooth.characteristic.correlated_color_temperature
    /// Unit is Kelvin with a resolution of 1.
    /// Minimum: 800
    /// Maximum: 65534
    /// Unit:
    /// org.bluetooth.unit.thermodynamic_temperature.kelvin
    /// A value of 0xFFFF represents ’value is not known’.
    /// uint16
    #[characteristic(uuid = "2AE9", read, notify)]
    pub(crate) cct: u16,

    /// uuid: 0x2AFF
    /// name: Luminous Flux
    /// id: org.bluetooth.characteristic.luminous_flux
    /// Unit is lumen with a resolution of 1
    /// Minimum: 0
    /// Maximum: 65534
    /// Represented values: M = 1, d = 0, b = 0
    /// Unit: org.bluetooth.unit.luminous_flux.lumen
    /// A value of 0xFFFF represents ’value is not known’.
    /// All other values are Prohibited.
    /// uint16
    #[characteristic(uuid = "2AFF", read, notify)]
    pub(crate) lux: u16,

    #[characteristic(uuid = "a0e4a2ba-0000-8000-0000-00805f9b34fb", read, write, notify)]
    pub(crate) timeout: u32,
}

#[nrf_softdevice::gatt_service(uuid = "ac866789-aaaa-eeee-a329-969d4bc8621e")]
pub(crate) struct SpiExpanderService {
    #[characteristic(uuid = "0000A001-0000-1000-8000-00805F9B34FB", write)]
    pub(crate) mosi: [u8; 64],

    #[characteristic(uuid = "0000A002-0000-1000-8000-00805F9B34FB", read, notify)]
    pub(crate) miso: [u8; 64],

    #[characteristic(uuid = "0000A003-0000-1000-8000-00805F9B34FB", read, notify)]
    pub(crate) cs: u8,

    #[characteristic(uuid = "0000A004-0000-1000-8000-00805F9B34FB", read, notify)]
    pub(crate) command: u8,

    #[characteristic(uuid = "0000A005-0000-1000-8000-00805F9B34FB", write, read, notify)]
    pub(crate) lock: u8,

    #[characteristic(uuid = "0000A006-0000-1000-8000-00805F9B34FB", read, notify)]
    pub(crate) power: bool,
}

#[nrf_softdevice::gatt_server]
pub(crate) struct BleServer {
    pub(crate) dis: DeviceInformationService,
    pub(crate) adc: AdcService,
    pub(crate) bme280: Bme280Service,
    pub(crate) accelerometer: AccelerometerService,
    pub(crate) color: ColorService,
    pub(crate) spi_expander: SpiExpanderService,
}
