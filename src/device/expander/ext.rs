use embassy_nrf::gpio::{AnyPin, Output};

pub(crate) trait Expander {
    fn select(&mut self, num: u8);
}

impl Expander for [&mut Output<'_, AnyPin>; 3] {
    fn select(&mut self, num: u8) {
        let flags = [
            num & (1 << 0) != 0,
            num & (1 << 1) != 0,
            num & (1 << 2) != 0,
        ];

        self.iter_mut().zip(flags).for_each(|(pin, flag)| {
            if flag {
                pin.set_high();
            } else {
                pin.set_low();
            }
        });
    }
}
