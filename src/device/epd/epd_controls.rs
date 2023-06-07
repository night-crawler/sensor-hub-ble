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

        // Transfer the command over spi
        self.write(&[command.address()]).await
    }

    async fn data(&mut self, data: &[u8]) -> Result<(), CustomSpimError> {
        self.dc.set_high();

        for (index, val) in data.iter().copied().enumerate() {
            // Transfer data one u8 at a time over spi
            info!("Writing {}", index);
            self.write( &[val]).await?;
        }
        // self.write(data).await
        Ok(())
    }

    async fn cmd_with_data<T: Command>(&mut self, command: T, data: &[u8]) -> Result<(), CustomSpimError> {
        self.cmd(command).await?;
        self.data(data).await
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

        // transfer spi data
        // Be careful!! Linux has a default limit of 4096 bytes per spi transfer
        // see https://raspberrypi.stackexchange.com/questions/65595/spi-transfer-fails-with-buffer-size-greater-than-4096
        self.interface.write(data).await?;

        // deactivate spi with cs high
        self.cs.set_high();

        Ok(())
    }

    async fn wait_until_idle(&mut self, is_busy_low: bool) {
        if is_busy_low {
            info!("wait_until_idle (high)");
            self.busy.wait_for_high().await;
        } else {
            info!("wait_until_idle (low)");
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

    async fn reset(&mut self, initial_delay: u32, duration: u32) {
        self.rst.set_high();
        Timer::after(Duration::from_micros(initial_delay as u64)).await;
        self.rst.set_low();

        Timer::after(Duration::from_micros(duration as u64)).await;
        self.rst.set_high();
        //TODO: the upstream libraries always sleep for 200ms here
        // 10ms works fine with just for the 7in5_v2 but this needs to be validated for other devices
        Timer::after(Duration::from_micros(200_000)).await;

        info!("Reset complete");
    }
}