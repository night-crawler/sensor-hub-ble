#[nrf_softdevice::gatt_service(uuid = "180A")]
pub(crate) struct DeviceInformationService {
    #[characteristic(uuid = "2a19", read, notify)]
    pub(crate) battery_level: u8,

    #[characteristic(uuid = "2A1C", read, notify)]
    pub(crate) temp: f32,
}


#[nrf_softdevice::gatt_service(uuid = "9e7312e0-2354-11eb-9f10-fbc30a62cf38")]
pub(crate) struct FooService {
    #[characteristic(uuid = "9e7312e0-2354-11eb-9f10-fbc30a63cf38", read, write, notify, indicate)]
    pub(crate) foo: u16,
}

#[nrf_softdevice::gatt_server]
pub(crate) struct BleServer {
    pub(crate) dis: DeviceInformationService,
    pub(crate) foo: FooService,
}
