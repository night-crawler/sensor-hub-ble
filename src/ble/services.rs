
#[nrf_softdevice::gatt_service(uuid = "180A")]
pub(crate) struct DeviceInformationService {
    #[characteristic(uuid = "2a19", read, notify)]
    pub(crate) battery_level: i16,

    #[characteristic(uuid = "2A6E", read, notify)]
    pub(crate) temp: i16,

    #[characteristic(uuid = "2BDE", read, notify)]
    pub(crate) debug: [u8; 64],
}


#[nrf_softdevice::gatt_service(uuid = "5c853275-723b-4754-a329-969d8bc8121d")]
pub(crate) struct AdcService {
    #[characteristic(uuid = "2B18", read, notify, indicate)]
    pub(crate) voltage1: i32,
}

#[nrf_softdevice::gatt_server]
pub(crate) struct BleServer {
    pub(crate) dis: DeviceInformationService,
    pub(crate) adc: AdcService,
}
