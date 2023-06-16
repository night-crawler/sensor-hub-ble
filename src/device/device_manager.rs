use core::mem;

use crate::common::bitbang;
use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::config::{HfclkSource, LfclkSource};
use embassy_nrf::gpio::{AnyPin, Pin};
use embassy_nrf::interrupt::typelevel::Interrupt;
use embassy_nrf::interrupt::typelevel::SAADC;
use embassy_nrf::interrupt::typelevel::SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0;
use embassy_nrf::interrupt::typelevel::SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1;
use embassy_nrf::interrupt::typelevel::SPIM2_SPIS2_SPI2;
use embassy_nrf::interrupt::Priority;
use embassy_nrf::saadc::{AnyInput, ChannelConfig, Input, Resistor, Saadc};
use embassy_nrf::spim;
use embassy_nrf::twim::{self};
use embassy_nrf::{bind_interrupts, peripherals, saadc, Peripherals};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use rclite::Arc;

use crate::common::device::error::DeviceError;
use crate::common::device::led_animation::{led_animation_task, LedState, LedStateAnimation, LED};

bind_interrupts!(pub(crate) struct Irqs {
    SAADC => saadc::InterruptHandler;
    SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0 => twim::InterruptHandler<peripherals::TWISPI0>;
    SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1 => twim::InterruptHandler<peripherals::TWISPI1>;
    SPIM2_SPIS2_SPI2 => spim::InterruptHandler<peripherals::SPI2>;
});

pub(crate) struct I2CPins<T> {
    pub(crate) twim: T,
    pub(crate) sda: AnyPin,
    pub(crate) scl: AnyPin,
    pub(crate) config: twim::Config,
}

pub(crate) struct BitbangI2CPins {
    pub(crate) sda: AnyPin,
    pub(crate) scl: AnyPin,
    pub(crate) config: bitbang::i2c::Config,
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

pub(crate) struct DeviceManager {
    pub(crate) spawner: Spawner,
    pub(crate) saadc: Saadc<'static, 8>,
    // pub(crate) i2c0: I2CPins<TWISPI0>,
    pub(crate) spi2: Arc<Mutex<ThreadModeRawMutex, SpiTxPins<peripherals::SPI2>>>,
    pub(crate) epd_control_pins: Arc<Mutex<ThreadModeRawMutex, EpdControlPins>>,
    pub(crate) bbi2c0: Arc<Mutex<ThreadModeRawMutex, BitbangI2CPins>>,
    pub(crate) bbi2c_exp: Arc<Mutex<ThreadModeRawMutex, BitbangI2CPins>>,
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
        LED.lock()
            .await
            .init(board.P0_22, board.P0_16, board.P0_24, board.P0_08);
        info!("Successfully Initialized LED");

        let mut led = LED.lock().await;
        led.blink_short(LedState::Purple).await;

        SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0::set_priority(Priority::P2);
        SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1::set_priority(Priority::P2);
        SPIM2_SPIS2_SPI2::set_priority(Priority::P3);
        info!("Successfully set interrupt priorities");

        let mut spim_conf = spim::Config::default();
        spim_conf.frequency = spim::Frequency::K500;

        let spi_tx_pins = SpiTxPins {
            spim: board.SPI2,
            sck: board.P0_21.degrade(),
            mosi: board.P0_23.degrade(),
            config: spim_conf,
        };

        let epd_control_pins = EpdControlPins {
            cs: board.P0_17.degrade(),
            dc: board.P0_15.degrade(),
            busy: board.P0_19.degrade(),
            rst: board.P0_13.degrade(),
        };

        led.blink_short(LedState::Purple).await;

        let bbi2c0 = BitbangI2CPins {
            scl: board.P1_11.degrade(),
            sda: board.P1_12.degrade(),
            config: Default::default(),
        };

        let bbi2c_exp = BitbangI2CPins {
            scl: board.P1_06.degrade(),
            sda: board.P1_04.degrade(),
            config: Default::default(),
        };

        let saadc = Self::init_adc(
            [
                board.P0_02.degrade_saadc(), // AIN0
                board.P0_03.degrade_saadc(), // AIN1 AIN.BAT
                board.P0_04.degrade_saadc(), // AIN2
                board.P0_05.degrade_saadc(), // AIN3
                board.P0_28.degrade_saadc(), // AIN4
                board.P0_29.degrade_saadc(), // AIN5
                board.P0_30.degrade_saadc(), // AIN6
                board.P0_31.degrade_saadc(), // AIN7
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
            epd_control_pins: Arc::new(Mutex::new(epd_control_pins)),
            spi2: Arc::new(Mutex::new(spi_tx_pins)),
            spawner,
            saadc,
            bbi2c0: Arc::new(Mutex::new(bbi2c0)),
            bbi2c_exp: Arc::new(Mutex::new(bbi2c_exp)),
        })
    }

    fn init_adc<const N: usize>(pins: [AnyInput; N], adc: peripherals::SAADC) -> Saadc<'static, N> {
        let config = saadc::Config::default();

        let mut channel_configs: [ChannelConfig; N] = unsafe { mem::zeroed() };
        for (index, pin) in pins.into_iter().enumerate() {
            let mut channel_cfg = ChannelConfig::single_ended(pin.degrade_saadc());
            channel_cfg.resistor = Resistor::PULLDOWN;
            channel_configs[index] = channel_cfg;
        }

        SAADC::set_priority(Priority::P3);
        let saadc = Saadc::new(adc, Irqs, config, channel_configs);
        saadc
    }
}

#[embassy_executor::task]
async fn set_watchdog_task() {
    loop {
        LedStateAnimation::blink(
            &[LedState::Blue],
            Duration::from_millis(100),
            Duration::from_secs(0),
        );
        Timer::after(Duration::from_secs(1)).await;
    }
}
