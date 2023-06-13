use embassy_nrf::gpio::{AnyPin, Input, Output, Pin as GpioPin};
use embassy_nrf::spim;
use embassy_time::{Duration, Timer};

use crate::common::compat::spi::SpimWrapper;

#[derive(Debug, defmt::Format)]
pub enum Error {
    Bus,
    NoData,
}

#[derive(defmt::Format, Clone, Copy)]
pub enum BitOrder {
    MSBFirst,
    LSBFirst,
}

impl Default for BitOrder {
    fn default() -> Self {
        BitOrder::MSBFirst
    }
}

#[derive(Copy, Clone)]
pub struct Config {
    /// Overread character.
    ///
    /// When doing bidirectional transfers, if the TX buffer is shorter than the RX buffer,
    /// this byte will be transmitted in the MOSI line for the left-over bytes.
    pub orc: u8,
    pub mode: spim::Mode,
    pub delay_duration: Duration,
    pub bit_order: BitOrder,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            orc: 0,
            mode: spim::MODE_0,
            delay_duration: Duration::from_hz(9000),
            bit_order: BitOrder::MSBFirst,
        }
    }
}

pub struct SpiBb<'d, SCK = AnyPin, MOSI = AnyPin, MISO = AnyPin> where SCK: GpioPin + 'd, MISO: GpioPin + 'd, MOSI: GpioPin + 'd {
    sck: Output<'d, SCK>,
    mosi: Option<Output<'d, MOSI>>,
    miso: Option<Input<'d, MISO>>,
    config: Config,
}

impl<'d, SCK, MOSI, MISO> SpiBb<'d, SCK, MOSI, MISO> where SCK: GpioPin + 'd, MISO: GpioPin + 'd, MOSI: GpioPin + 'd {
    /// ```
    /// let mut spi = <bitbang::spi::SpiBb>::new_txonly(
    ///     Output::new(&mut spi_pins.sck, Level::Low, OutputDrive::Standard),
    ///     Output::new(&mut spi_pins.mosi, Level::Low, OutputDrive::Standard),
    ///     bitbang::spi::Config::default()
    /// );
    /// ```
    pub fn new_txonly(
        sck: Output<'d, SCK>,
        mosi: Output<'d, MOSI>,
        config: Config,
    ) -> SpiBb<'d, SCK, MOSI, MISO> {
        SpiBb::<'d, SCK, MOSI, MISO>::new(sck, Some(mosi), None, config)
    }

    pub fn new(
        sck: Output<'d, SCK>,
        mosi: Option<Output<'d, MOSI>>,
        miso: Option<Input<'d, MISO>>,
        config: Config,
    ) -> Self {
        let mut spi = Self {
            sck,
            mosi,
            miso,
            config,
        };

        match config.mode.polarity {
            spim::Polarity::IdleLow => spi.sck.set_low(),
            spim::Polarity::IdleHigh => spi.sck.set_high(),
        }

        spi
    }

    #[inline]
    fn read_bit(&self, current_value: &mut u8) {
        *current_value <<= 1;
        if let Some(miso) = &self.miso {
            *current_value |= miso.is_high() as u8;
        }
    }

    #[inline]
    fn set_clk_high(&mut self) {
        self.sck.set_high();
    }

    #[inline]
    fn set_clk_low(&mut self) {
        self.sck.set_low();
    }

    #[inline]
    async fn wait_for_timer(&self) {
        Timer::after(self.config.delay_duration).await;
    }

    async fn exchange_byte(&mut self, read_byte: &mut u8, write_byte: u8) -> Result<(), Error> {
        for bit_offset in 0..8 {
            let out_bit = match self.config.bit_order {
                BitOrder::MSBFirst => (write_byte >> (7 - bit_offset)) & 0b1,
                BitOrder::LSBFirst => (write_byte >> bit_offset) & 0b1,
            };

            if let Some(mosi) = &mut self.mosi {
                if out_bit == 1 {
                    mosi.set_high();
                } else {
                    mosi.set_low();
                }
            }

            if self.config.mode == spim::MODE_0 {
                self.wait_for_timer().await;
                self.set_clk_high();
                self.read_bit(read_byte);
                self.wait_for_timer().await;
                self.set_clk_low();
            } else if self.config.mode == spim::MODE_1 {
                self.set_clk_high();
                self.wait_for_timer().await;
                self.read_bit(read_byte);
                self.set_clk_low();
                self.wait_for_timer().await;
            } else if self.config.mode == spim::MODE_2 {
                self.wait_for_timer().await;
                self.set_clk_low();
                self.read_bit(read_byte);
                self.wait_for_timer().await;
                self.set_clk_high();
            } else if self.config.mode == spim::MODE_3 {
                self.set_clk_low();
                self.wait_for_timer().await;
                self.read_bit(read_byte);
                self.set_clk_high();
                self.wait_for_timer().await;
            } else {
                panic!("Unknown mode");
            }
        }

        Ok(())
    }
}

impl<'d, SCK, MOSI, MISO> SpimWrapper<Error> for SpiBb<'d, SCK, MOSI, MISO>
    where SCK: GpioPin + 'd, MISO: GpioPin + 'd, MOSI: GpioPin + 'd {
    async fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Error> {
        let mut fake_read = 0u8;
        for index in 0..read.len().max(write.len()) {
            let read_byte = read.get_mut(index).unwrap_or(&mut fake_read);
            let write_byte = *write.get(index).unwrap_or(&self.config.orc);
            self.exchange_byte(read_byte, write_byte).await?;
        }
        Ok(())
    }

    async fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        for &write_byte in data {
            self.exchange_byte(&mut 0, write_byte).await?;
        }
        Ok(())
    }

    async fn read(&mut self, data: &mut [u8]) -> Result<(), Error> {
        for read_byte in data.iter_mut() {
            self.exchange_byte(read_byte, self.config.orc).await?;
        }
        Ok(())
    }
}
