use defmt::info;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_storage_async::nor_flash::{NorFlash, ReadNorFlash};
use futures::pin_mut;
use nrf_softdevice::Flash;

use crate::common::ble::{FLASH_MANAGER, SERVER};
use crate::common::device::config::{CONFIG_FLASH_SIZE, FLASH_PAGE_SIZE, INIT_TOKEN};
use crate::common::device::error::{DeviceError, FlashManagerError};

pub(crate) struct FlashManager {
    flash: Mutex<ThreadModeRawMutex, Flash>,
    offset: u32,
    token_offset: u32,
    last_data: Mutex<ThreadModeRawMutex, CalibrationData>,
}


#[derive(Default, defmt::Format, Clone, Copy)]
pub(crate) struct CalibrationData {
    pub(crate) version: usize,
    pub(crate) bme_humidity: f32,
    pub(crate) bme_pressure: f32,
    pub(crate) bme_temperature: f32,
}

impl CalibrationData {
    pub(crate) fn equal_ignoring_version(&self, other: &Self) -> bool {
        self.bme_humidity == other.bme_humidity
            && self.bme_pressure == other.bme_pressure
            && self.bme_temperature == other.bme_temperature
    }
}

trait FlashExt {
    async fn write_calibration_data(&mut self, offset: u32, data: &CalibrationData) -> Result<(), FlashManagerError>;
    async fn read_calibration_data(&mut self, offset: u32) -> Result<CalibrationData, FlashManagerError>;
}

impl FlashExt for Flash {
    async fn write_calibration_data(&mut self, offset: u32, data: &CalibrationData) -> Result<(), FlashManagerError> {
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(&data.version.to_le_bytes());
        buf[4..8].copy_from_slice(&data.bme_humidity.to_le_bytes());
        buf[8..12].copy_from_slice(&data.bme_pressure.to_le_bytes());
        buf[12..16].copy_from_slice(&data.bme_temperature.to_le_bytes());

        self.write(offset, &buf).await?;

        Ok(())
    }

    async fn read_calibration_data(&mut self, offset: u32) -> Result<CalibrationData, FlashManagerError> {
        let mut buf = [0u8; 16];
        self.read(offset, &mut buf).await?;

        let version = usize::from_le_bytes(buf.clone_subarray(0));
        let bme_humidity = f32::from_le_bytes(buf.clone_subarray(4));
        let bme_pressure = f32::from_le_bytes(buf.clone_subarray(8));
        let bme_temperature = f32::from_le_bytes(buf.clone_subarray(12));

        Ok(CalibrationData {
            bme_humidity,
            bme_pressure,
            bme_temperature,
            version,
        })
    }
}

impl FlashManager {
    pub fn new(flash: Flash) -> Self {
        let offset = 100 * FLASH_PAGE_SIZE as u32;
        Self {
            flash: Mutex::new(flash),
            offset,
            token_offset: offset + CONFIG_FLASH_SIZE as u32,
            last_data: Mutex::new(CalibrationData::default()),
        }
    }

    pub(crate) async fn init(&self) -> Result<(), FlashManagerError> {
        if self.is_initialized().await? {
            *self.last_data.lock().await = self.flash.lock().await.read_calibration_data(self.offset).await?;
            return Ok(());
        }

        let mut flash = self.flash.lock().await;
        pin_mut!(flash);

        let total_size = CONFIG_FLASH_SIZE as u32 + INIT_TOKEN.len() as u32;
        // must be one 4096 page in total
        assert_eq!(total_size, FLASH_PAGE_SIZE as u32);
        let end_addr = self.offset + total_size;

        info!("Initializing flash for the first time: {:x} - {:x}", self.offset, end_addr);
        flash.erase(self.offset, end_addr).await?;
        info!("Erased flash: {:x} - {:x}", self.offset, end_addr);

        flash.write(self.token_offset, &INIT_TOKEN).await?;
        info!("Wrote init token from offset {:x}", self.token_offset);

        let cd = CalibrationData::default();
        flash.write_calibration_data(self.offset, &cd).await?;
        *self.last_data.lock().await = cd;

        Ok(())
    }

    pub(crate) async fn is_initialized(&self) -> Result<bool, FlashManagerError> {
        let mut flash = self.flash.lock().await;
        pin_mut!(flash);

        let mut buf = [0u8; 4];
        flash.read(self.token_offset, &mut buf).await?;

        Ok(buf == INIT_TOKEN)
    }

    pub(crate) async fn read_calibration_data(&self) -> Result<CalibrationData, FlashManagerError> {
        let mut flash = self.flash.lock().await;
        pin_mut!(flash);

        let mut cd = self.last_data.lock().await;
        *cd = flash.read_calibration_data(self.offset).await?;

        Ok(cd.clone())
    }

    pub(crate) async fn write_calibration_data(&self, next_data: &CalibrationData) -> Result<(), FlashManagerError> {
        info!("Writing calibration data: {:?}", next_data);
        let existing_data = self.read_calibration_data().await?;
        if existing_data.equal_ignoring_version(next_data) {
            info!("Calibration data is the same, skipping write");
            return Ok(());
        }
        if existing_data.version >= next_data.version {
            return Err(FlashManagerError::RaceCondition(existing_data.version, next_data.version));
        }

        let mut flash = self.flash.lock().await;
        pin_mut!(flash);

        // the whole page needs to be erased before write
        flash.erase(self.offset, self.token_offset + INIT_TOKEN.len() as u32).await?;

        flash.write_calibration_data(self.offset, next_data).await?;
        flash.write(self.token_offset, &INIT_TOKEN).await?;
        *self.last_data.lock().await = next_data.clone();

        info!("Wrote calibration data: {:?}", next_data);

        Ok(())
    }

    pub(crate) async fn get_last_calibration_data(&self) -> CalibrationData {
        *self.last_data.lock().await
    }
}

trait ClonedSlice<T> {
    fn clone_subarray<const S: usize>(&self, offset: usize) -> [T; S];
}

impl<T> ClonedSlice<T> for [T] where T: Copy + Default {
    fn clone_subarray<const S: usize>(&self, offset: usize) -> [T; S] {
        let mut arr = [Default::default(); S];
        arr.copy_from_slice(&self[offset..offset + S]);
        arr
    }
}


pub(crate) async fn copy_calibration_data_from_flash() -> Result<(), DeviceError> {
    let server = SERVER.get();
    let calibration_data = FLASH_MANAGER.get().get_last_calibration_data().await;
    server.bme280.humidity_offset_set(&calibration_data.bme_humidity.to_le_bytes())?;
    server.bme280.pressure_offset_set(&calibration_data.bme_pressure.to_le_bytes())?;
    server.bme280.temperature_offset_set(&calibration_data.bme_temperature.to_le_bytes())?;

    info!("Calibration data copied from flash: {:?}", calibration_data);

    Ok(())
}