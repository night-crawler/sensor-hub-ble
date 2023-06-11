use embassy_nrf::gpio::{AnyPin, Flex, Output, Pin as GpioPin};
use embassy_time::{Duration, Timer};

use crate::common::compat::i2c::I2CWrapper;

#[derive(Copy, Clone)]
pub struct Config {
    pub delay_duration: Duration,
}

pub enum Error {
    NoAck,
    InvalidData,
}


pub struct I2C<'d, SCL = AnyPin, SDA = AnyPin> where SCL: GpioPin + 'd, SDA: GpioPin + 'd {
    scl: Output<'d, SCL>,
    sda: Flex<'d, SDA>,
    config: Config,
}


impl<'d, SCL, SDA> I2C<'d, SCL, SDA> where SCL: GpioPin + 'd, SDA: GpioPin + 'd {
    pub fn new(
        scl: Output<'d, SCL>,
        sda: Flex<'d, SDA>,
        config: Config,
    ) -> Self {
        let mut i2c = Self {
            scl,
            sda,
            config,
        };
        i2c
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
            self.wait();
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
    async fn check_ack(&mut self) -> Result<(), Error> {
        if !self.i2c_is_ack().await {
            Err(Error::NoAck)
        } else {
            Ok(())
        }
    }

    #[inline]
    async fn read_from_slave(&mut self, input: &mut [u8]) -> Result<(), Error> {
        for i in 0..input.len() {
            let should_send_ack = i != (input.len() - 1);
            input[i] = self.i2c_read_byte(should_send_ack).await;
        }
        Ok(())
    }

    #[inline]
    async fn write_to_slave(&mut self, output: &[u8]) -> Result<(), Error> {
        for &byte in output {
            self.i2c_write_byte(byte).await;
            self.check_ack().await?;
        }
        Ok(())
    }
}

impl<'d, SCL, SDA> I2CWrapper<Error> for I2C<'d, SCL, SDA> where SCL: GpioPin + 'd, SDA: GpioPin + 'd {
    async fn write_read(&mut self, address: u8, wr_buffer: &[u8], rd_buffer: &mut [u8]) -> Result<(), Error> {
        if wr_buffer.is_empty() || rd_buffer.is_empty() {
            return Err(Error::InvalidData);
        }

        // ST
        self.i2c_start();

        // SAD + W
        self.i2c_write_byte((address << 1) | 0x0).await;
        self.check_ack().await?;

        self.write_to_slave(wr_buffer).await?;

        // SR
        self.i2c_start().await;

        // SAD + R
        self.i2c_write_byte((address << 1) | 0x1).await;
        self.check_ack().await?;

        self.read_from_slave(rd_buffer).await?;

        // SP
        self.i2c_stop().await;
        Ok(())
    }

    async fn write(&mut self, address: u8, buffer: &[u8]) -> Result<(), Error> {
        // ST
        self.i2c_start().await;

        // SAD + W
        self.i2c_write_byte((address << 1) | 0x0).await;
        self.check_ack().await?;

        self.write_to_slave(buffer).await?;

        // SP
        self.i2c_stop().await;

        Ok(())
    }

    async fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Error> {
        if buffer.is_empty() {
            return Ok(());
        }

        // ST
        self.i2c_start().await;

        // SAD + R
        self.i2c_write_byte((address << 1) | 0x1).await;
        self.check_ack().await?;

        self.read_from_slave(buffer).await?;

        // SP
        self.i2c_stop().await;

        Ok(())
    }
}
