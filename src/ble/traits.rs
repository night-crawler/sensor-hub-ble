use crate::common::device::error::DeviceError;
use nrf_softdevice::ble::Connection;

pub(crate) trait ConnectionEventHandler {
    fn handle(&self, connection: &Connection) -> Result<(), DeviceError>;
}
