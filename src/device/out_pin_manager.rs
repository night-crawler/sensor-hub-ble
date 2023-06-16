use embassy_nrf::gpio::{AnyPin, Level, Output, OutputDrive};
use embassy_time::{Duration, Timer};
use smallvec::SmallVec;

#[derive(Default)]
pub struct OutPinManager {
    pub pins: SmallVec<[Output<'static, AnyPin>; 16]>,
}

impl OutPinManager {
    pub(crate) fn register<P>(&mut self, pin: P) -> Option<&mut Output<'static, AnyPin>>
    where
        P: Into<AnyPin>,
    {
        if self.pins.len() + 1 > 16 {
            return None;
        }
        self.pins
            .push(Output::new(pin.into(), Level::Low, OutputDrive::Standard));
        self.pins.last_mut()
    }
    pub(crate) fn get(&mut self, index: usize) -> Option<&mut Output<'static, AnyPin>> {
        self.pins.get_mut(index)
    }

    pub(crate) fn all_high(&mut self) {
        for pin in self.pins.iter_mut() {
            pin.set_high();
        }
    }

    pub(crate) fn all_low(&mut self) {
        for pin in self.pins.iter_mut() {
            pin.set_low();
        }
    }

    pub(crate) async fn sweep(&mut self, timeout: u64) {
        for (index, pin) in self.pins.iter_mut().enumerate() {
            pin.set_low();
            let index = index + 1;

            for _ in 0..index {
                pin.set_high();
                Timer::after(Duration::from_millis(timeout * 2)).await;
                pin.set_low();
                Timer::after(Duration::from_millis(timeout)).await;
            }
        }
        self.all_low();
    }
}
