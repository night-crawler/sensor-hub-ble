use core::mem::MaybeUninit;

use defmt::unwrap;
use embassy_executor::{Spawner, SpawnError};
use embassy_nrf::gpio::{AnyPin, Level, Output, OutputDrive};
use embassy_nrf::interrupt::Priority;
use embassy_nrf::Peripherals;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use heapless::{Arc, arc_pool};
use smallvec::SmallVec;

arc_pool!(P: Mutex<ThreadModeRawMutex, OutPinManager>);


#[derive(Default)]
pub struct OutPinManager {
    pins: SmallVec<[Output<'static, AnyPin>; 16]>,
}


impl OutPinManager {
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
                Timer::after(Duration::from_millis(timeout * 2)).await;
                pin.set_low();
                Timer::after(Duration::from_millis(timeout)).await;
            }
        }
        self.all_low();
    }
}


pub(crate) struct DeviceManager {
    pub(crate) pin_group1: Arc<P>,
    pub(crate) pin_group2: Arc<P>,
    pub(crate) pin_group3: Arc<P>,

    pub(crate) spawner: Spawner,
}

fn prepare_nrf_peripherals() -> Peripherals {
    let mut config = embassy_nrf::config::Config::default();
    // config.hfclk_source = HfclkSource::ExternalXtal;
    // config.lfclk_source = LfclkSource::Synthesized;
    config.gpiote_interrupt_priority = Priority::P2;
    config.time_interrupt_priority = Priority::P2;
    embassy_nrf::init(config)
}

impl DeviceManager {
    pub(crate) async fn new(spawner: Spawner) -> Self {
        let board = prepare_nrf_peripherals();

        let mut pin_group1 = OutPinManager::default();
        pin_group1.register(board.P0_02);
        // pin_group1.register(board.P0_00);
        // pin_group1.register(board.P0_01);
        // pin_group1.register(board.P0_07);
        // pin_group1.register(board.P0_08);
        // pw.register(board.P0_09);
        // pw.register(board.P0_10);
        // pin_group1.register(board.P0_11);
        // pin_group1.register(board.P0_12);
        // pin_group1.register(board.P0_14);
        // pin_group1.register(board.P0_15);
        // pin_group1.register(board.P0_16);

        let mut pin_group2 = OutPinManager::default();
        // pin_group2.register(board.P0_17);
        // pin_group2.register(board.P0_19);
        // pin_group2.register(board.P0_20);
        // pin_group2.register(board.P0_21);
        // pin_group2.register(board.P0_22);
        // pin_group2.register(board.P0_23);
        // pin_group2.register(board.P0_24);
        // pin_group2.register(board.P0_25);
        pin_group2.register(board.P0_06);
        pin_group2.register(board.P0_26);
        pin_group2.register(board.P0_30);

        let mut pin_group3 = OutPinManager::default();
        // pin_group3.register(board.P1_00);
        // pin_group3.register(board.P1_01);
        // pin_group3.register(board.P1_02);
        // pin_group3.register(board.P1_03);
        // pin_group3.register(board.P1_04);
        // pin_group3.register(board.P1_05);
        // pin_group3.register(board.P1_06);
        // pin_group3.register(board.P1_07);
        // pin_group3.register(board.P1_08);
        // pin_group3.register(board.P1_09);
        // pin_group3.register(board.P1_10);

        pin_group1.all_low();
        pin_group2.all_low();
        pin_group3.all_low();

        static mut MEMORY: [u8; 2048] = [0; 2048];
        let res = unsafe {
            P::grow(&mut MEMORY)
        };

        Self {
            spawner,
            pin_group1: unwrap!(P::alloc(Mutex::new(pin_group1)).ok()),
            pin_group2: unwrap!(P::alloc(Mutex::new(pin_group2)).ok()),
            pin_group3: unwrap!(P::alloc(Mutex::new(pin_group3)).ok()),
        }
    }

    pub(crate) async fn sp(&self) -> Result<(), SpawnError> {
        // self.spawner.spawn(blink_task(self.pin_group1.clone()))
        self.spawner.spawn(blink_task(self.pin_group2.clone(), self.pin_group1.clone()))
        // self.spawner.spawn(blink_task(self.pin_group3.clone()))
    }
}


#[embassy_executor::task(pool_size = 3)]
async fn blink_task(pin_group: Arc<P>, control: Arc<P>) -> ! {
    loop {
        pin_group.lock().await.all_low();
        let mut control_pin = control.lock().await;
        control_pin.all_low();

        let mut g = pin_group.lock().await;
        for (index, pin) in g.pins.iter_mut().enumerate() {
            pin.set_high();

            for _ in 0..index + 1 {
                control_pin.all_high();
                Timer::after(Duration::from_millis(10)).await;
                control_pin.all_low();
                Timer::after(Duration::from_millis(10)).await;
            }

            pin.set_low();
            Timer::after(Duration::from_millis(50)).await;
        }
    }
}
