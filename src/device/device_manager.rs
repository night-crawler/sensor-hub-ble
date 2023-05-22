use core::mem::MaybeUninit;

use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_nrf::config::{HfclkSource, LfclkSource};
use embassy_nrf::interrupt::Priority;
use embassy_nrf::Peripherals;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use heapless::{Arc, arc_pool};

use crate::common::device::error::DeviceError;
use crate::common::device::led_animation::{LED, led_animation_task, LedState, LedStateAnimation};
use crate::common::device::out_pin_manager::OutPinManager;

arc_pool!(P: Mutex<ThreadModeRawMutex, OutPinManager>);


pub(crate) struct DeviceManager {
    pub(crate) pin_group1: Arc<P>,
    pub(crate) spawner: Spawner,
}

fn prepare_nrf_peripherals() -> Peripherals {
    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = HfclkSource::ExternalXtal;
    config.lfclk_source = LfclkSource::ExternalXtal;
    config.gpiote_interrupt_priority = Priority::P2;
    config.time_interrupt_priority = Priority::P2;
    embassy_nrf::init(config)
}

impl DeviceManager {
    pub(crate) async fn new(spawner: Spawner) -> Result<Self, DeviceError> {
        let board = prepare_nrf_peripherals();

        LED.lock().await.init(
            board.P0_26,
            board.P0_30,
            board.P0_06,
            board.P0_17,
        );

        let _ = spawner.spawn(led_animation_task());
        let _ = spawner.spawn(set_watchdog_task());

        LedStateAnimation::sweep_indices(u16::MAX, Duration::from_millis(50), Duration::from_millis(10));

        let mut pin_group1 = OutPinManager::default();
        pin_group1.register(board.P0_02);

        static mut MEMORY: [u8; 2048] = [0; 2048];
        let res = unsafe {
            P::grow(&mut MEMORY)
        };
        LedStateAnimation::blink_long(&[LedState::White]);

        Ok(Self {
            spawner,
            pin_group1: unwrap!(P::alloc(Mutex::new(pin_group1)).ok()),
        })
    }
}

#[embassy_executor::task]
async fn set_watchdog_task() {
    loop {
        LedStateAnimation::blink(&[LedState::Tx], Duration::from_millis(100), Duration::from_secs(0));
        Timer::after(Duration::from_secs(1)).await;
    }
}
