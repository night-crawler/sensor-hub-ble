use defmt::info;
use embassy_nrf::gpio::{AnyPin, Input, Output};
use embassy_time::{Duration, Timer};

use crate::common::device::epd::interface::DisplayInterface;
use crate::common::device::epd::traits::Command;
use crate::common::device::error::CustomSpimError;
use crate::common::device::ext::spi::SpimWrapper;

pub(crate) struct EpdControls<'a, I: SpimWrapper> {
    interface: &'a mut I,
    busy: Input<'a, AnyPin>,
    cs: Output<'a, AnyPin>,
    dc: Output<'a, AnyPin>,
    rst: Output<'a, AnyPin>,
    delay_us: u64
}

impl<'a, I: SpimWrapper> EpdControls<'a, I> {
    pub(crate) fn new(
        interface: &'a mut I,
        busy: Input<'a, AnyPin>,
        cs: Output<'a, AnyPin>,
        dc: Output<'a, AnyPin>,
        rst: Output<'a, AnyPin>,
    ) -> Self {
        Self {
            interface,
            busy,
            cs,
            dc,
            rst,
            delay_us: 10_000
        }
    }
}

impl<'a, I: SpimWrapper> DisplayInterface for EpdControls<'a, I> {
    async fn cmd<T: Command>(&mut self, command: T) -> Result<(), CustomSpimError> {
        self.dc.set_low();
        self.cs.set_low();

        Timer::after(Duration::from_micros(1)).await;
        self.interface.write(&[command.address()]).await?;
        Timer::after(Duration::from_micros(1)).await;

        self.cs.set_high();
        Ok(())
    }

    async fn data(&mut self, data: &[u8]) -> Result<(), CustomSpimError> {
        self.dc.set_high();
        self.cs.set_low();

        Timer::after(Duration::from_micros(1)).await;

        for (index, b) in data.iter().copied().enumerate() {
            self.interface.write(&[b]).await?;
        }
        Timer::after(Duration::from_micros(1)).await;
        self.cs.set_high();
        Ok(())
    }

    async fn cmd_with_data<T: Command>(&mut self, command: T, data: &[u8]) -> Result<(), CustomSpimError> {
        self.cmd(command).await?;
        self.data(data).await?;
        Ok(())
    }

    async fn data_x_times(&mut self, val: u8, repetitions: u32) -> Result<(), CustomSpimError> {
        self.dc.set_high();
        // Transfer data (u8) over spi
        for _ in 0..repetitions {
            self.write(&[val]).await?;
        }
        Ok(())
    }

    async fn write(&mut self, data: &[u8]) -> Result<(), CustomSpimError> {
        // activate spi with cs low
        self.cs.set_low();
        Timer::after(Duration::from_micros(1)).await;


        // transfer spi data
        // Be careful!! Linux has a default limit of 4096 bytes per spi transfer
        // see https://raspberrypi.stackexchange.com/questions/65595/spi-transfer-fails-with-buffer-size-greater-than-4096
        self.interface.write(data).await?;

        Timer::after(Duration::from_micros(1)).await;

        // deactivate spi with cs high
        self.cs.set_high();

        Ok(())
    }

    async fn wait_until_idle(&mut self, is_busy_low: bool) {
        if is_busy_low {
            self.busy.wait_for_high().await;
        } else {
            self.busy.wait_for_low().await
        }
    }

    async fn wait_until_idle_with_cmd<T: Command>(&mut self, is_busy_low: bool, status_command: T) -> Result<(), CustomSpimError> {
        self.cmd(status_command).await?;
        if self.delay_us > 0 {
            Timer::after(Duration::from_micros(self.delay_us)).await
        }
        while self.is_busy(is_busy_low) {
            self.cmd(status_command).await?;
            if self.delay_us > 0 {
                Timer::after(Duration::from_micros(self.delay_us)).await
            }
        }
        Ok(())
    }

    fn is_busy(&self, is_busy_low: bool) -> bool {
        (is_busy_low && self.busy.is_low())
            || (!is_busy_low && self.busy.is_high())
    }

    async fn reset(&mut self, _initial_delay: u32, _duration: u32) {
        self.rst.set_high();
        Timer::after(Duration::from_millis(20)).await;
        self.rst.set_low();
        Timer::after(Duration::from_micros(2)).await;
        self.rst.set_high();
        Timer::after(Duration::from_micros(20)).await;

        info!("Reset complete");
    }
}