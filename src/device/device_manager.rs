use core::mem;
use core::mem::MaybeUninit;

use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, interrupt, peripherals, Peripherals, saadc};
use embassy_nrf::config::{HfclkSource, LfclkSource};
use embassy_nrf::gpio::{AnyPin, Pin};
use embassy_nrf::interrupt::{Interrupt, InterruptExt, Priority};
use embassy_nrf::peripherals::{SAADC, TWISPI0, TWISPI1};
use embassy_nrf::saadc::{AnyInput, ChannelConfig, Input, Resistor, Saadc};
use embassy_nrf::twim::{self};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use heapless::{Arc, arc_pool};

use crate::common::device::error::DeviceError;
use crate::common::device::led_animation::{LED, led_animation_task, LedState, LedStateAnimation};
use crate::common::device::out_pin_manager::OutPinManager;

arc_pool!(P: Mutex<ThreadModeRawMutex, OutPinManager>);


bind_interrupts!(pub(crate) struct Irqs {
    SAADC => saadc::InterruptHandler;
    SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0 => twim::InterruptHandler<peripherals::TWISPI0>;
    SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1 => twim::InterruptHandler<peripherals::TWISPI1>;
});


pub(crate) struct I2CPins<T> {
    pub(crate) twim: T,
    pub(crate) sda: AnyPin,
    pub(crate) scl: AnyPin,
    pub(crate) config: twim::Config,
}


pub(crate) struct DeviceManager {
    pub(crate) pin_group1: Arc<P>,
    pub(crate) spawner: Spawner,
    pub(crate) saadc: Saadc<'static, 6>,
    pub(crate) i2c0: I2CPins<TWISPI0>,
    pub(crate) i2c1: I2CPins<TWISPI1>,
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

        let mut led = LED.lock().await;

        led.blink_short(LedState::Purple).await;

        unsafe {
            interrupt::SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0::steal().set_priority(Priority::P2);
            interrupt::SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1::steal().set_priority(Priority::P2);
        };

        let i2c0 = I2CPins {
            twim: board.TWISPI0,
            sda: board.P1_11.degrade(),
            scl: board.P1_12.degrade(),
            config: Default::default(),
        };

        let i2c1 = I2CPins {
            twim: board.TWISPI1,
            sda: board.P1_13.degrade(),
            scl: board.P1_14.degrade(),
            config: Default::default(),
        };

        led.blink_short(LedState::Purple).await;

        let mut saadc = Self::init_adc(
            [
                board.P0_02.degrade_saadc(),
                board.P0_03.degrade_saadc(),
                board.P0_28.degrade_saadc(),
                board.P0_29.degrade_saadc(),
                board.P0_04.degrade_saadc(),
                board.P0_05.degrade_saadc()
            ],
            board.SAADC,
        );
        saadc.calibrate().await;

        led.blink_short(LedState::Purple).await;

        let _ = spawner.spawn(led_animation_task());
        let _ = spawner.spawn(set_watchdog_task());

        led.blink_short(LedState::Purple).await;

        let mut pin_group1 = OutPinManager::default();
        pin_group1.register(board.P1_15);

        led.blink_short(LedState::Purple).await;

        static mut MEMORY: [u8; 1024] = [0; 1024];
        led.blink_short(LedState::Purple).await;

        let res = unsafe {
            P::grow(&mut MEMORY)
        };

        led.blink_short(LedState::Green).await;

        Ok(Self {
            i2c0,
            i2c1,
            spawner,
            pin_group1: unwrap!(P::alloc(Mutex::new(pin_group1)).ok()),
            saadc,
        })
    }

    fn init_adc<const N: usize>(pins: [AnyInput; N], adc: SAADC) -> Saadc<'static, N> {
        let config = saadc::Config::default();

        let mut channel_configs: [ChannelConfig; N] = unsafe { mem::zeroed() };
        for (index, pin) in pins.into_iter().enumerate() {
            let mut channel_cfg = ChannelConfig::single_ended(pin.degrade_saadc());
            channel_cfg.resistor = Resistor::PULLDOWN;
            channel_configs[index] = channel_cfg;
        }

        unsafe { interrupt::SAADC::steal() }.set_priority(Priority::P3);
        let saadc = Saadc::new(adc, Irqs, config, channel_configs);
        saadc
    }
}

#[embassy_executor::task]
async fn set_watchdog_task() {
    loop {
        LedStateAnimation::blink(&[LedState::Tx], Duration::from_millis(100), Duration::from_secs(0));
        Timer::after(Duration::from_secs(1)).await;
    }
}
