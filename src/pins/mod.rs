use embassy_nrf::config::{HfclkSource, LfclkSource};
use embassy_nrf::gpio::{AnyPin, Level, Output, OutputDrive};
use embassy_nrf::interrupt::Priority;
use embassy_nrf::Peripherals;

use smallvec::SmallVec;
use embassy_time::{Duration, Timer};

#[derive(Default)]
pub(crate) struct OutPinWrapper {
    pins: SmallVec<[Output<'static, AnyPin>; 16]>,
}


impl OutPinWrapper {
    pub(crate) fn register<P>(&mut self, pin: P) -> Option<&mut Output<'static, AnyPin>> where P: Into<AnyPin> {
        if self.pins.len() + 1 > 16 {
            return None;
        }
        self.pins.push(Output::new(pin.into(), Level::Low, OutputDrive::Standard));
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
                Timer::after(Duration::from_millis(timeout)).await;
                pin.set_low();
                Timer::after(Duration::from_millis(timeout)).await;
            }
        }
    }
}


pub(crate) struct DeviceManager {
    pub(crate) pin_group1: OutPinWrapper,
    pub(crate) pin_group2: OutPinWrapper,
    pub(crate) pin_group3: OutPinWrapper,
}

impl DeviceManager {
    pub(crate) fn new() -> Self {
        let mut config = embassy_nrf::config::Config::default();
        // config.hfclk_source = HfclkSource::ExternalXtal;
        // config.lfclk_source = LfclkSource::Synthesized;
        config.gpiote_interrupt_priority = Priority::P2;
        config.time_interrupt_priority = Priority::P2;
        let board = embassy_nrf::init(config);

        let mut pin_group1 = OutPinWrapper::default();
        pin_group1.register(board.P0_00);
        pin_group1.register(board.P0_01);
        pin_group1.register(board.P0_02);
        pin_group1.register(board.P0_03);
        pin_group1.register(board.P0_04);
        pin_group1.register(board.P0_05);
        pin_group1.register(board.P0_06);
        pin_group1.register(board.P0_07);
        pin_group1.register(board.P0_08);
        // pw.register(board.P0_09);
        // pw.register(board.P0_10);
        pin_group1.register(board.P0_11);
        pin_group1.register(board.P0_12);
        pin_group1.register(board.P0_13);
        pin_group1.register(board.P0_14);
        pin_group1.register(board.P0_15);
        pin_group1.register(board.P0_16);


        let mut pin_group2 = OutPinWrapper::default();
        pin_group2.register(board.P0_17);
        pin_group2.register(board.P0_19);
        pin_group2.register(board.P0_20);
        pin_group2.register(board.P0_21);
        pin_group2.register(board.P0_22);
        pin_group2.register(board.P0_23);
        pin_group2.register(board.P0_24);
        pin_group2.register(board.P0_25);
        pin_group2.register(board.P0_26);
        pin_group2.register(board.P0_27);
        pin_group2.register(board.P0_28);
        pin_group2.register(board.P0_29);
        pin_group2.register(board.P0_30);
        pin_group2.register(board.P0_31);

        let mut pin_group3 = OutPinWrapper::default();
        pin_group3.register(board.P1_00);
        pin_group3.register(board.P1_01);
        pin_group3.register(board.P1_02);
        pin_group3.register(board.P1_03);
        pin_group3.register(board.P1_04);
        pin_group3.register(board.P1_05);
        pin_group3.register(board.P1_06);
        pin_group3.register(board.P1_07);
        pin_group3.register(board.P1_08);
        pin_group3.register(board.P1_09);
        pin_group3.register(board.P1_10);
        pin_group3.register(board.P1_11);
        pin_group3.register(board.P1_12);
        pin_group3.register(board.P1_13);
        pin_group3.register(board.P1_14);
        pin_group3.register(board.P1_15);


        Self {
            pin_group1,
            pin_group2,
            pin_group3,
        }
    }
}

