// use core::marker::PhantomData;
//
// use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
// use embassy_sync::mutex::Mutex;
// use embedded_hal_async::i2c::{ErrorType, I2c, Operation, SevenBitAddress};
//
// pub(crate) struct SharedI2c<'a, I, E> {
//     bus: &'a Mutex<ThreadModeRawMutex, I>,
//     _phantom_data: PhantomData<E>,
// }
//
// impl<'a, I, E> SharedI2c<'a, I, E> {
//     pub(crate) fn new(bus: &'a Mutex<ThreadModeRawMutex, I>) -> Self {
//         Self { bus, _phantom_data: PhantomData }
//     }
// }
//
// impl<'a, I, E> ErrorType for SharedI2c<'a, I, E> where I: ErrorType<Error=E> + I2c, E: embedded_hal_async::i2c::Error {
//     type Error = E;
// }
//
// impl<'a, I, E> I2c for SharedI2c<'a, I, E> where I: I2c + ErrorType<Error=E>, E: embedded_hal_async::i2c::Error {
//     async fn read(&mut self, address: SevenBitAddress, read: &mut [u8]) -> Result<(), <SharedI2c<'a, I, E> as ErrorType>::Error> {
//         todo!()
//     }
//
//     async fn write(&mut self, address: SevenBitAddress, write: &[u8]) -> Result<(), <SharedI2c<'a, I, E> as ErrorType>::Error> {
//         todo!()
//     }
//
//     async fn write_read(&mut self, address: SevenBitAddress, write: &[u8], read: &mut [u8]) -> Result<(), <SharedI2c<'a, I, E> as ErrorType>::Error> {
//         todo!()
//     }
//
//     async fn transaction(&mut self, address: SevenBitAddress, operations: &mut [Operation<'_>]) -> Result<(), <SharedI2c<'a, I, E> as ErrorType>::Error> {
//         todo!()
//     }
// }

use core::future::Future;
use core::ops::DerefMut;

use embassy_nrf::gpio::{Flex, Level, Output, OutputDrive, Pull};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_hal_async::i2c::{ErrorType, I2c, Operation, SevenBitAddress};

use crate::common::bitbang::i2c::{BitbangI2C, BitbangI2CError};
use crate::common::device::device_manager::BitbangI2CPins;

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

        let mut sda = Flex::new(&mut i2c_pins_mut_ref.sda);
        sda.set_as_input_output(Pull::None, OutputDrive::Standard0Disconnect1);
        let mut i2c = BitbangI2C::new(
            Output::new(&mut i2c_pins_mut_ref.scl, Level::High, OutputDrive::Standard0Disconnect1),
            sda,
            Default::default(),
        );

        match op {
            Op::Write(address, write) => i2c.write(address, write).await,
            Op::Read(address, read) => i2c.read(address, read).await,
            Op::WriteRead(address, write, read) => i2c.write_read(address, write, read).await,
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
        address: SevenBitAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), <SharedBitbangI2cPins<'a> as ErrorType>::Error> {
        todo!()
    }
}
