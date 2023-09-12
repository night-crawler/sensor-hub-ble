use core::fmt::Formatter;
use embassy_nrf::gpio::{AnyPin, Flex, Output, Pin as GpioPin};
use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::{
    Error, ErrorKind, ErrorType, I2c, NoAcknowledgeSource, Operation, SevenBitAddress,
};

#[derive(Copy, Clone)]
pub struct Config {
    pub delay_duration: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self { delay_duration: Duration::from_hz(10_000) }
    }
}

#[derive(Debug, defmt::Format)]
pub enum BitbangI2CError {
    NoAck,
    WriteTimeout,
    ReadTimeout,
    WriteReadTimeout,
    InvalidData,
}

impl core::fmt::Display for BitbangI2CError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct BitbangI2C<'d, SCL = AnyPin, SDA = AnyPin>
where
    SCL: GpioPin + 'd,
    SDA: GpioPin + 'd,
{
    scl: Output<'d, SCL>,
    sda: Flex<'d, SDA>,
    config: Config,
}

impl<'d, SCL, SDA> BitbangI2C<'d, SCL, SDA>
where
    SCL: GpioPin + 'd,
    SDA: GpioPin + 'd,
{
    pub fn new(scl: Output<'d, SCL>, sda: Flex<'d, SDA>, config: Config) -> Self {
        Self { scl, sda, config }
    }

    async fn wait(&self) {
        Timer::after(self.config.delay_duration).await
    }

    async fn i2c_start(&mut self) {
        self.scl.set_high();
        self.sda.set_high();
        self.wait().await;

        self.sda.set_low();
        self.wait().await;

        self.scl.set_low();
        self.wait().await;
    }

    async fn i2c_stop(&mut self) {
        self.scl.set_high();
        self.wait().await;

        self.sda.set_high();
        self.wait().await;
    }

    async fn i2c_is_ack(&mut self) -> bool {
        self.sda.set_high();
        self.scl.set_high();
        self.wait().await;

        let ack = self.sda.is_low();

        self.scl.set_low();
        self.sda.set_low();
        self.wait().await;

        ack
    }

    async fn i2c_read_byte(&mut self, should_send_ack: bool) -> u8 {
        let mut byte: u8 = 0;

        self.sda.set_high();

        for bit_offset in 0..8 {
            self.scl.set_high();
            self.wait().await;

            if self.sda.is_high() {
                byte |= 1 << (7 - bit_offset);
            }

            self.scl.set_low();
            self.wait().await;
        }

        if should_send_ack {
            self.sda.set_low();
        } else {
            self.sda.set_high();
        }

        self.scl.set_high();
        self.wait().await;

        self.scl.set_low();
        self.sda.set_low();
        self.wait().await;

        byte
    }

    async fn i2c_write_byte(&mut self, byte: u8) {
        for bit_offset in 0..8 {
            let out_bit = (byte >> (7 - bit_offset)) & 0b1;

            if out_bit == 1 {
                self.sda.set_high();
            } else {
                self.sda.set_low();
            }

            self.scl.set_high();
            self.wait().await;

            self.scl.set_low();
            self.sda.set_low();
            self.wait().await;
        }
    }

    #[inline]
    async fn check_ack(&mut self) -> Result<(), BitbangI2CError> {
        if !self.i2c_is_ack().await { Err(BitbangI2CError::NoAck) } else { Ok(()) }
    }

    #[inline]
    async fn read_from_slave(&mut self, input: &mut [u8]) -> Result<(), BitbangI2CError> {
        for i in 0..input.len() {
            let should_send_ack = i != (input.len() - 1);
            input[i] = self.i2c_read_byte(should_send_ack).await;
        }
        Ok(())
    }

    #[inline]
    async fn write_to_slave(&mut self, output: &[u8]) -> Result<(), BitbangI2CError> {
        for &byte in output {
            self.i2c_write_byte(byte).await;
            self.check_ack().await?;
        }
        Ok(())
    }
}

impl<'d, SCL, SDA> ErrorType for BitbangI2C<'d, SCL, SDA>
where
    SCL: 'd + GpioPin,
    SDA: 'd + GpioPin,
{
    type Error = BitbangI2CError;
}

impl Error for BitbangI2CError {
    fn kind(&self) -> ErrorKind {
        match self {
            BitbangI2CError::NoAck => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Data),
            BitbangI2CError::InvalidData => ErrorKind::Other,
            BitbangI2CError::WriteTimeout => ErrorKind::Other,
            BitbangI2CError::ReadTimeout => ErrorKind::Other,
            BitbangI2CError::WriteReadTimeout => ErrorKind::Other,
        }
    }
}

impl<'d, SCL, SDA> I2c for BitbangI2C<'d, SCL, SDA>
where
    SCL: GpioPin + 'd,
    SDA: GpioPin + 'd,
{
    async fn read(
        &mut self,
        address: SevenBitAddress,
        read: &mut [u8],
    ) -> Result<(), <BitbangI2C<'d, SCL, SDA> as ErrorType>::Error> {
        if read.is_empty() {
            return Ok(());
        }

        // ST
        self.i2c_start().await;

        // SAD + R
        self.i2c_write_byte((address << 1) | 0x1).await;
        self.check_ack().await?;

        self.read_from_slave(read).await?;

        // SP
        self.i2c_stop().await;

        Ok(())
    }

    async fn write(
        &mut self,
        address: SevenBitAddress,
        write: &[u8],
    ) -> Result<(), <BitbangI2C<'d, SCL, SDA> as ErrorType>::Error> {
        // ST
        self.i2c_start().await;

        // SAD + W
        self.i2c_write_byte(address << 1).await;
        self.check_ack().await?;

        self.write_to_slave(write).await?;

        // SP
        self.i2c_stop().await;

        Ok(())
    }

    async fn write_read(
        &mut self,
        address: SevenBitAddress,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), <BitbangI2C<'d, SCL, SDA> as ErrorType>::Error> {
        if write.is_empty() || read.is_empty() {
            return Err(BitbangI2CError::InvalidData);
        }

        // ST
        self.i2c_start().await;

        // SAD + W
        self.i2c_write_byte(address << 1).await;
        self.check_ack().await?;

        self.write_to_slave(write).await?;

        // SR
        self.i2c_start().await;

        // SAD + R
        self.i2c_write_byte((address << 1) | 0x1).await;
        self.check_ack().await?;

        self.read_from_slave(read).await?;

        // SP
        self.i2c_stop().await;
        Ok(())
    }

    async fn transaction(
        &mut self,
        _address: SevenBitAddress,
        _operations: &mut [Operation<'_>],
    ) -> Result<(), <BitbangI2C<'d, SCL, SDA> as ErrorType>::Error> {
        todo!()
    }
}
