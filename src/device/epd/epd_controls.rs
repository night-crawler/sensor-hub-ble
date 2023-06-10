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
    async fn send_command<T: Command>(&mut self, command: T) -> Result<(), CustomSpimError> {
        self.dc.set_low();

        // Transfer the command over spi
        self.write(&[command.address()]).await
    }

    async fn send_data(&mut self, data: &[u8]) -> Result<(), CustomSpimError> {
        self.dc.set_high();
        self.write(data).await
    }

    async fn send_command_with_data<T: Command>(&mut self, command: T, data: &[u8]) -> Result<(), CustomSpimError> {
        self.send_command(command).await?;
        self.send_data(data).await
    }

    async fn send_data_x_times(&mut self, val: u8, repetitions: u32) -> Result<(), CustomSpimError> {
        self.dc.set_high();
        // Transfer data (u8) over spi
        for _ in 0..repetitions {
            self.write(&[val]).await?;
        }
        Ok(())
    }

    async fn write(&mut self, data: &[u8]) -> Result<(), CustomSpimError> {
        self.cs.set_low();
        self.interface.write(data).await?;
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

    async fn reset(&mut self, initial_delay: u32, duration: u32) {
        self.rst.set_high();
        Timer::after(Duration::from_micros(initial_delay as u64)).await;
        self.rst.set_low();

        Timer::after(Duration::from_micros(duration as u64)).await;
        self.rst.set_high();
        Timer::after(Duration::from_micros(200_000)).await;

        info!("Reset complete");
    }
}