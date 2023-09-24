use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, peripherals, Peripherals, saadc};
use embassy_nrf::config::{HfclkSource, LfclkSource};
use embassy_nrf::gpio::{AnyPin, Level, Output, OutputDrive, Pin};
use embassy_nrf::interrupt::Priority;
use embassy_nrf::interrupt::typelevel::Interrupt;
use embassy_nrf::interrupt::typelevel::SAADC;
use embassy_nrf::interrupt::typelevel::SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0;
use embassy_nrf::interrupt::typelevel::SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1;
use embassy_nrf::interrupt::typelevel::SPIM2_SPIS2_SPI2;
use embassy_nrf::interrupt::typelevel::SPIM3;
use embassy_nrf::saadc::{AnyInput, Input};
use embassy_nrf::spim;
use embassy_nrf::twim::{self};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use rclite::Arc;

use crate::common::bitbang;
use crate::common::device::error::DeviceError;

bind_interrupts!(pub(crate) struct Irqs {
    SAADC => saadc::InterruptHandler;
    SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0 => twim::InterruptHandler<peripherals::TWISPI0>;
    SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1 => twim::InterruptHandler<peripherals::TWISPI1>;
    SPIM3 => spim::InterruptHandler<peripherals::SPI3>;
    SPIM2_SPIS2_SPI2 => spim::InterruptHandler<peripherals::SPI2>;
});


pub(crate) struct ExpanderPins<SPI, TWIM> {
    pub(crate) sda: AnyPin,
    pub(crate) scl: AnyPin,

    pub(crate) miso: AnyPin,
    pub(crate) mosi: AnyPin,
    pub(crate) sck: AnyPin,

    pub(crate) power_switch: Output<'static, AnyPin>,
    pub(crate) a0: Output<'static, AnyPin>,
    pub(crate) a1: Output<'static, AnyPin>,
    pub(crate) a2: Output<'static, AnyPin>,

    pub(crate) spi_peripheral: SPI,
    pub(crate) i2c_peripheral: TWIM,
    pub(crate) spim_config: spim::Config,
    pub(crate) i2c_config: twim::Config,
}

pub(crate) struct ButtonPins {
    pub(crate) top_left: AnyPin,
    pub(crate) top_right: AnyPin,
    pub(crate) bottom_left: AnyPin,
}

#[allow(unused)]
pub(crate) struct I2CPins<T> {
    pub(crate) twim: T,
    pub(crate) sda: AnyPin,
    pub(crate) scl: AnyPin,
    pub(crate) config: twim::Config,
}

pub(crate) struct SaadcPins<const N: usize> {
    pub(crate) adc: peripherals::SAADC,
    pub(crate) pins: [AnyInput; N],
    pub(crate) pw_switch: AnyPin,
    pub(crate) bat_switch: AnyPin,
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
    pub(crate) saadc_pins: Arc<Mutex<ThreadModeRawMutex, SaadcPins<8>>>,
    // pub(crate) i2c0: I2CPins<TWISPI0>,
    pub(crate) spi2_pins: Arc<Mutex<ThreadModeRawMutex, SpiTxPins<peripherals::SPI2>>>,
    pub(crate) epd_control_pins: Arc<Mutex<ThreadModeRawMutex, EpdControlPins>>,
    pub(crate) bbi2c0_pins: Arc<Mutex<ThreadModeRawMutex, BitbangI2CPins>>,
    pub(crate) button_pins: ButtonPins,
    pub(crate) expander_pins: Arc<Mutex<ThreadModeRawMutex, ExpanderPins<peripherals::SPI3, peripherals::TWISPI1>>>,
}

fn prepare_nrf_peripherals() -> Peripherals {
    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = HfclkSource::ExternalXtal;
    config.lfclk_source = LfclkSource::ExternalXtal;
    config.gpiote_interrupt_priority = Priority::P2;
    config.time_interrupt_priority = Priority::P2;
    // let a = unsafe {
    //     &*nrf52840_pac::UICR::ptr()
    // };
    // a.regout0.write(|w| w.vout().variant(nrf52840_pac::uicr::regout0::VOUT_A::_3V3));

    embassy_nrf::init(config)
}

impl DeviceManager {
    pub(crate) async fn new(spawner: Spawner) -> Result<Self, DeviceError> {
        let board = prepare_nrf_peripherals();

        // TWISPI0 is stolen
        SAADC::set_priority(Priority::P3);
        SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0::set_priority(Priority::P2);
        SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1::set_priority(Priority::P2);
        SPIM3::set_priority(Priority::P7);

        SPIM2_SPIS2_SPI2::set_priority(Priority::P3);
        info!("Successfully set interrupt priorities");

        let mut epd_spim_conf = spim::Config::default();
        epd_spim_conf.frequency = spim::Frequency::M2;

        let spi_tx_pins = SpiTxPins {
            spim: board.SPI2,
            sck: board.P0_21.degrade(),
            mosi: board.P0_23.degrade(),
            config: epd_spim_conf,
        };

        let epd_control_pins = EpdControlPins {
            cs: board.P0_17.degrade(),
            dc: board.P0_15.degrade(),
            busy: board.P0_19.degrade(),
            rst: board.P0_13.degrade(),
        };

        let bbi2c0 = BitbangI2CPins {
            scl: board.P1_11.degrade(),
            sda: board.P1_12.degrade(),
            config: Default::default(),
        };

        let saadc_pins = SaadcPins {
            adc: board.SAADC,
            pins: [
                board.P0_02.degrade_saadc(), // AIN0
                board.P0_04.degrade_saadc(), // AIN2
                board.P0_05.degrade_saadc(), // AIN3
                board.P0_28.degrade_saadc(), // AIN4
                board.P0_29.degrade_saadc(), // AIN5
                board.P0_30.degrade_saadc(), // AIN6
                board.P0_31.degrade_saadc(), // AIN7
                board.P0_03.degrade_saadc(), // AIN1 AIN.BAT
            ],
            pw_switch: board.P1_07.degrade(),
            bat_switch: board.P1_08.degrade(),
        };

        let button_pins = ButtonPins {
            top_left: board.P1_01.degrade(),
            top_right: board.P1_05.degrade(),
            bottom_left: board.P1_03.degrade(),
        };


        let expander_pins = ExpanderPins {
            sda: board.P1_04.degrade(),
            scl: board.P1_06.degrade(),

            miso: board.P0_16.degrade(),
            mosi: board.P0_14.degrade(),
            sck: board.P0_20.degrade(),

            power_switch: Output::new(board.P0_22.degrade(), Level::Low, OutputDrive::Disconnect0Standard1),
            a0: Output::new(board.P0_24.degrade(), Level::Low, OutputDrive::Disconnect0Standard1),
            a1: Output::new(board.P0_25.degrade(), Level::Low, OutputDrive::Disconnect0Standard1),
            a2: Output::new(board.P1_02.degrade(), Level::Low, OutputDrive::Disconnect0Standard1),

            spim_config: Default::default(),
            i2c_config: Default::default(),
            i2c_peripheral: board.TWISPI1,
            spi_peripheral: board.SPI3,
        };

        Ok(Self {
            epd_control_pins: Arc::new(Mutex::new(epd_control_pins)),
            spi2_pins: Arc::new(Mutex::new(spi_tx_pins)),
            saadc_pins: Arc::new(Mutex::new(saadc_pins)),
            bbi2c0_pins: Arc::new(Mutex::new(bbi2c0)),

            button_pins,
            expander_pins: Arc::new(Mutex::new(expander_pins)),
        })
    }
}
