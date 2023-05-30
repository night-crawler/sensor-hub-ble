use embassy_nrf::twim;
use embassy_nrf::twim::Twim;

use crate::common::device::error::CustomI2CError;

pub trait I2CWrapper {
    async fn write_read(&mut self, address: u8, wr_buffer: &[u8], rd_buffer: &mut [u8]) -> Result<(), CustomI2CError>;
    async fn write(&mut self, address: u8, buffer: &[u8]) -> Result<(), CustomI2CError>;
    async fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), CustomI2CError>;
}

impl<'a, T: twim::Instance> I2CWrapper for Twim<'a, T> {
    async fn write_read(&mut self, address: u8, wr_buffer: &[u8], rd_buffer: &mut [u8]) -> Result<(), CustomI2CError> {
        // https://docs.embassy.dev/embassy-nrf/git/nrf52840/index.html#easydma-considerations
        self.write_read(address, wr_buffer, rd_buffer).await?;
        Ok(())
    }

    async fn write(&mut self, address: u8, buffer: &[u8]) -> Result<(), CustomI2CError> {
        self.write(address, buffer).await?;
        Ok(())
    }

    async fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), CustomI2CError> {
        self.read(address, buffer).await?;
        Ok(())
    }
}

