use core::mem;

use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, interrupt, peripherals, Peripherals, saadc};
use embassy_nrf::config::{HfclkSource, LfclkSource};
use embassy_nrf::gpio::{AnyPin, Pin};
use embassy_nrf::interrupt::{Interrupt, InterruptExt, Priority};
use embassy_nrf::peripherals::{SAADC, SPI3};
use embassy_nrf::saadc::{AnyInput, ChannelConfig, Input, Resistor, Saadc};
use embassy_nrf::spim;
use embassy_nrf::twim::{self};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};

use crate::common::device::error::DeviceError;
use crate::common::device::led_animation::{LED, led_animation_task, LedState, LedStateAnimation};

bind_interrupts!(pub(crate) struct Irqs {
    SAADC => saadc::InterruptHandler;
    SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0 => twim::InterruptHandler<peripherals::TWISPI0>;
    SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1 => twim::InterruptHandler<peripherals::TWISPI1>;
    SPIM3 => spim::InterruptHandler<peripherals::SPI3>;
});


pub(crate) struct I2CPins<T> {
    pub(crate) twim: T,
    pub(crate) sda: AnyPin,
    pub(crate) scl: AnyPin,
    pub(crate) config: twim::Config,
}

pub(crate) struct SpiTxPins<T> {
    pub(crate) spim: T,
    pub(crate) sck: AnyPin,
    pub(crate) mosi: AnyPin,
    pub(crate) config: spim::Config,
}

pub(crate) struct EpdControlPins {
    pub(crate) cs: AnyPin,
    pub(crate) dc: AnyPin,
    pub(crate) busy: AnyPin,
    pub(crate) rst: AnyPin,
}

use rclite::Arc;

pub(crate) struct DeviceManager {
    pub(crate) spawner: Spawner,
    pub(crate) saadc: Saadc<'static, 5>,
    // pub(crate) i2c0: I2CPins<TWISPI0>,
    // pub(crate) i2c1: I2CPins<TWISPI1>,
    pub(crate) spi3: Arc<Mutex<ThreadModeRawMutex, SpiTxPins<SPI3>>>,
    pub(crate) epd_control_pins: Arc<Mutex<ThreadModeRawMutex, EpdControlPins>>,
}

fn prepare_nrf_peripherals() -> Peripherals {
    // spim::In
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
            board.P0_22,
            board.P0_16,
            board.P0_24,
            board.P0_08,
        );
        info!("Successfully Initialized LED");

        let mut led = LED.lock().await;
        led.blink_short(LedState::Purple).await;

        unsafe {
            interrupt::SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0::steal().set_priority(Priority::P2);
            interrupt::SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1::steal().set_priority(Priority::P2);
            interrupt::SPIM3::steal().set_priority(Priority::P2);
        };
        info!("Successfully set interrupt priorities");

        // mosi 5
        // sclk 6

        // let mut a: spim::Spim<SPI3> = spim::Spim::new_txonly(board.SPI3, Irqs,  board.P1_15, board.P1_14, spim_conf);
        // let busy = embassy_nrf::gpio::Input::new(board.P1_13, Pull::None);  // 1
        // let cs = Output::new(board.P1_12.degrade(), Level::High,OutputDrive::Standard);  // 4
        // let dc = Output::new(board.P1_11, Level::High,OutputDrive::Standard);  // 3
        // let rst = Output::new(board.P0_05, Level::High,OutputDrive::Standard);  // 2

        let spi_tx_pins = SpiTxPins {
            spim: board.SPI3,
            sck: board.P0_21.degrade(),
            mosi: board.P0_23.degrade(),
            config: spim::Config::default(),
        };

        let epd_control_pins = EpdControlPins {
            cs: board.P0_17.degrade(),
            dc: board.P0_15.degrade(),
            busy: board.P0_19.degrade(),
            rst: board.P0_13.degrade(),
        };


        // EpdControls::new(cs, busy, dc, rst).unwrap(

        // let qwe = embassy_nrf::timer::Timer::new(board.TIMER4);
        // let qweqwe = Epd2in13::new(&mut a, cs, busy, dc, rst, &mut qwe).unwrap();
        // qweqwe.ena

        led.blink_short(LedState::Purple).await;

        // Timer::after(Duration::from_secs(100)).await;


        // let i2c0 = I2CPins {
        //     twim: board.TWISPI0,
        //     sda: board.P1_11.degrade(),
        //     scl: board.P1_12.degrade(),
        //     config: Default::default(),
        // };
        //
        // let i2c1 = I2CPins {
        //     twim: board.TWISPI1,
        //     sda: board.P1_13.degrade(),
        //     scl: board.P1_14.degrade(),
        //     config: Default::default(),
        // };


        let mut saadc = Self::init_adc(
            [
                board.P0_02.degrade_saadc(),
                board.P0_03.degrade_saadc(),
                board.P0_28.degrade_saadc(),
                board.P0_29.degrade_saadc(),
                board.P0_04.degrade_saadc(),
                // board.P0_05.degrade_saadc()
            ],
            board.SAADC,
        );
        saadc.calibrate().await;
        info!("Successfully Initialized SAADC");

        led.blink_short(LedState::Purple).await;

        spawner.spawn(led_animation_task())?;
        spawner.spawn(set_watchdog_task())?;
        info!("Successfully spawned LED and Watchdog tasks");

        led.blink_short(LedState::Green).await;

        Ok(Self {
            // i2c0,
            // i2c1,
            epd_control_pins: Arc::new(Mutex::new(epd_control_pins)),
            spi3: Arc::new(Mutex::new(spi_tx_pins)),
            spawner,
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
        LedStateAnimation::blink(&[LedState::Blue], Duration::from_millis(100), Duration::from_secs(0));
        Timer::after(Duration::from_secs(1)).await;
    }
}
