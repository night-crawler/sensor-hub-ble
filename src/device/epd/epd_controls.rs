use core::marker::PhantomData;
use defmt::info;
use embassy_nrf::gpio::{AnyPin, Input, Output};
use embassy_time::{Duration, Timer};
use embedded_hal_async::spi::{SpiBusRead, SpiBusWrite};

use crate::common::device::epd::interface::DisplayInterface;
use crate::common::device::epd::traits::Command;

pub(crate) struct EpdControls<'a, I: SpiBusWrite + SpiBusRead> {
    interface: &'a mut I,
    busy: Input<'a, AnyPin>,
    cs: Output<'a, AnyPin>,
    dc: Output<'a, AnyPin>,
    rst: Output<'a, AnyPin>,
    delay_us: u64,
}

impl<'a, I: SpiBusWrite + SpiBusRead> EpdControls<'a, I> {
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
            delay_us: 10_000,
        }
    }
}

impl<'a, E, I> DisplayInterface<E> for EpdControls<'a, I>
where E: From<<I as embedded_hal_async::spi::ErrorType>::Error>,
      I: SpiBusWrite + SpiBusRead
{
    async fn send_command<T: Command>(&mut self, command: T) -> Result<(), E> {
        self.dc.set_low();
        self.write(&[command.address()]).await?;
        Ok(())
    }

    async fn send_data(&mut self, data: &[u8]) -> Result<(), E> {
        self.dc.set_high();
        self.write(data).await?;
        Ok(())
    }

    async fn send_command_with_data<T: Command>(&mut self, command: T, data: &[u8]) -> Result<(), E> {
        self.send_command(command).await?;
        self.send_data(data).await?;
        Ok(())
    }

    async fn send_data_x_times(&mut self, val: u8, repetitions: u32) -> Result<(), E> {
        self.dc.set_high();
        // Transfer data (u8) over spi
        for _ in 0..repetitions {
            self.write(&[val]).await?;
        }
        Ok(())
    }

    async fn write(&mut self, data: &[u8]) -> Result<(), E> {
        self.cs.set_low();
        Timer::after(Duration::from_micros(1)).await;

        self.interface.write(data).await?;

        Timer::after(Duration::from_micros(1)).await;

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

    async fn reset(&mut self, _initial_delay: u32, _duration: u32) {
        self.rst.set_high();
        Timer::after(Duration::from_millis(20)).await;
        self.rst.set_low();
        Timer::after(Duration::from_millis(2)).await;
        self.rst.set_high();
        Timer::after(Duration::from_millis(20)).await;

        info!("Reset complete");
    }
}