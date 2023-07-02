use core::future::Future;
use core::ops::DerefMut;

use embassy_nrf::gpio::{Flex, Level, Output, OutputDrive, Pull};
use embassy_nrf::peripherals::TWISPI0;
use embassy_nrf::twim;
use embassy_nrf::twim::{Error, Frequency, Twim};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_hal_async::i2c::{ErrorType, I2c, Operation, SevenBitAddress};

use crate::common::bitbang::i2c::{BitbangI2C, BitbangI2CError};
use crate::common::device::device_manager::{BitbangI2CPins, Irqs};

pub(crate) struct SharedBitbangI2cPins<'a> {
    pins: &'a Mutex<ThreadModeRawMutex, BitbangI2CPins>,
}

enum Op<'a> {
    Write(u8, &'a [u8]),
    Read(u8, &'a mut [u8]),
    WriteRead(u8, &'a [u8], &'a mut [u8]),
}

impl<'a> SharedBitbangI2cPins<'a> {
    pub(crate) fn new(pins: &'a Mutex<ThreadModeRawMutex, BitbangI2CPins>) -> Self {
        Self { pins }
    }

    // I have no fucking idea how to write a 'with' callback/trait for this usecase with async
    // this shit is fucking annoying
    // async fn with<C, F>(&self, mut cb: C)
    //     where for<'b> C: FnMut(& mut BitbangI2C<'b>) -> F + 'b, F: Future<Output=Result<(), BitbangI2CError>> + 'a
    // {

    async fn run_op<'b>(&self, op: Op<'b>) -> Result<(), BitbangI2CError> {
        let mut i2c_pins = self.pins.lock().await;
        let i2c_pins_mut_ref = i2c_pins.deref_mut();

        // let mut sda = Flex::new(&mut i2c_pins_mut_ref.sda);
        // sda.set_as_input_output(Pull::None, OutputDrive::Standard0Disconnect1);
        // let mut i2c = BitbangI2C::new(
        //     Output::new(&mut i2c_pins_mut_ref.scl, Level::High, OutputDrive::Standard0Disconnect1),
        //     sda,
        //     Default::default(),
        // );

        let mut config = twim::Config::default();
        config.scl_pullup = false;
        config.sda_pullup = false;
        config.frequency = Frequency::K400;
        let mut i2c = Twim::new(unsafe { TWISPI0::steal() }, Irqs, &mut i2c_pins_mut_ref.sda, &mut i2c_pins_mut_ref.scl, config);

        let result = match op {
            Op::Write(address, write) => i2c.write(address, write).await,
            Op::Read(address, read) => i2c.read(address, read).await,
            Op::WriteRead(address, write, read) => i2c.write_read(address, write, read).await,
        };

        match result {
            Ok(q) => Ok(q),
            Err(_) => Err(BitbangI2CError::NoAck)
        }
    }
}

impl<'a> ErrorType for SharedBitbangI2cPins<'a> {
    type Error = BitbangI2CError;
}

impl<'a> I2c for SharedBitbangI2cPins<'a> {
    async fn read(
        &mut self,
        address: SevenBitAddress,
        read: &mut [u8],
    ) -> Result<(), <SharedBitbangI2cPins<'a> as ErrorType>::Error> {
        self.run_op(Op::Read(address, read)).await
    }

    async fn write(
        &mut self,
        address: SevenBitAddress,
        write: &[u8],
    ) -> Result<(), <SharedBitbangI2cPins<'a> as ErrorType>::Error> {
        self.run_op(Op::Write(address, write)).await
    }

    async fn write_read(
        &mut self,
        address: SevenBitAddress,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), <SharedBitbangI2cPins<'a> as ErrorType>::Error> {
        self.run_op(Op::WriteRead(address, write, read)).await
    }

    async fn transaction(
        &mut self,
        _address: SevenBitAddress,
        _operations: &mut [Operation<'_>],
    ) -> Result<(), <SharedBitbangI2cPins<'a> as ErrorType>::Error> {
        todo!()
    }
}
