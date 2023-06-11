use embassy_nrf::spim::{Instance, Spim};

use crate::common::device::error::CustomSpimError;

pub trait SpimWrapper<E> {
    async fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), E>;
    async fn write(&mut self, data: &[u8]) -> Result<(), E>;
    async fn read(&mut self, data: &mut [u8]) -> Result<(), E>;
}

impl<'d, T: Instance> SpimWrapper<CustomSpimError> for Spim<'d, T> {
    async fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), CustomSpimError> {
        self.transfer(read, write).await?;
        Ok(())
    }

    async fn write(&mut self, data: &[u8]) -> Result<(), CustomSpimError> {
        self.write(data).await?;
        Ok(())
    }

    async fn read(&mut self, data: &mut [u8]) -> Result<(), CustomSpimError> {
        self.read(data).await?;
        Ok(())
    }
}
