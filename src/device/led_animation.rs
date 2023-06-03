use core::mem::transmute;
use defmt::debug;

use embassy_nrf::gpio::{AnyPin, Level, Output, OutputDrive};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use lazy_static::lazy_static;

use crate::common::device::config::{DURATION_LONG_MS, DURATION_SHORT_MS, LED_ANIMATION_QUEUE_LEN};

static CHANNEL: Channel<ThreadModeRawMutex, LedStateAnimation, LED_ANIMATION_QUEUE_LEN> = Channel::new();
static SHORT: Duration = Duration::from_millis(DURATION_SHORT_MS);
static LONG: Duration = Duration::from_millis(DURATION_LONG_MS);

lazy_static! {
    pub static ref LED: Mutex<ThreadModeRawMutex, LedManager> = {
        Mutex::new(LedManager::default())
    };
}


#[repr(u8)]
#[derive(Copy, Clone)]
pub enum LedState {
    Tx,
    Red,
    Green,
    Blue,
    Purple,
    Yellow,
    Cyan,
    White,
    TxOff,
    Off,
}

impl From<u8> for LedState {
    fn from(value: u8) -> Self {
        unsafe { transmute(value) }
    }
}


#[derive(Default)]
pub struct LedManager {
    red: Option<Output<'static, AnyPin>>,
    green: Option<Output<'static, AnyPin>>,
    blue: Option<Output<'static, AnyPin>>,

    tx: Option<Output<'static, AnyPin>>,
}

impl LedManager {
    pub fn init<P1, P2, P3, P4>(&mut self, red: P1, green: P2, blue: P3, tx: P4) where
        P1: Into<AnyPin>,
        P2: Into<AnyPin>,
        P3: Into<AnyPin>,
        P4: Into<AnyPin>,
    {
        self.red.replace(Output::new(red.into(), Level::High, OutputDrive::Standard0Disconnect1));
        self.green.replace(Output::new(green.into(), Level::High, OutputDrive::Standard0Disconnect1));
        self.blue.replace(Output::new(blue.into(), Level::High, OutputDrive::Standard0Disconnect1));
        self.tx.replace(Output::new(tx.into(), Level::High, OutputDrive::Standard0Disconnect1));
    }

    pub fn reset(&mut self) {
        self.red.as_mut().unwrap().set_high();
        self.green.as_mut().unwrap().set_high();
        self.blue.as_mut().unwrap().set_high();
        self.tx.as_mut().unwrap().set_high();
    }

    pub fn set_state(&mut self, color: LedState) {
        match color {
            LedState::Red => {
                self.red.as_mut().unwrap().set_low();
                self.green.as_mut().unwrap().set_high();
                self.blue.as_mut().unwrap().set_high();
            }
            LedState::Green => {
                self.red.as_mut().unwrap().set_high();
                self.green.as_mut().unwrap().set_low();
                self.blue.as_mut().unwrap().set_high();
            }
            LedState::Blue => {
                self.red.as_mut().unwrap().set_high();
                self.green.as_mut().unwrap().set_high();
                self.blue.as_mut().unwrap().set_low();
            }
            LedState::Purple => {
                self.red.as_mut().unwrap().set_low();
                self.green.as_mut().unwrap().set_high();
                self.blue.as_mut().unwrap().set_low();
            }
            LedState::Yellow => {
                self.red.as_mut().unwrap().set_low();
                self.green.as_mut().unwrap().set_low();
                self.blue.as_mut().unwrap().set_high();
            }
            LedState::Cyan => {
                self.red.as_mut().unwrap().set_high();
                self.green.as_mut().unwrap().set_low();
                self.blue.as_mut().unwrap().set_low();
            }
            LedState::White => {
                self.red.as_mut().unwrap().set_low();
                self.green.as_mut().unwrap().set_low();
                self.blue.as_mut().unwrap().set_low();
            }
            LedState::Off => {
                self.red.as_mut().unwrap().set_high();
                self.green.as_mut().unwrap().set_high();
                self.blue.as_mut().unwrap().set_high();
                self.tx.as_mut().unwrap().set_high();
            }
            LedState::Tx => {
                self.tx.as_mut().unwrap().set_low();
            }
            LedState::TxOff => {
                self.tx.as_mut().unwrap().set_high();
            }
        }
    }

    pub async fn blink(&mut self, state: LedState, on: Duration, off: Duration) {
        self.set_state(state);
        Timer::after(on).await;
        self.set_state(LedState::Off);
        Timer::after(off).await;
    }

    pub async fn blink_short(&mut self, state: LedState) {
        self.set_state(state);
        Timer::after(SHORT).await;
        self.set_state(LedState::Off);
        Timer::after(SHORT).await;
    }

    pub async fn blink_long(&mut self, state: LedState) {
        self.set_state(state);
        Timer::after(LONG).await;
        self.set_state(LedState::Off);
        Timer::after(LONG).await;
    }
}


pub enum LedStateAnimation {
    Sweep(u16, Duration, Duration),
    Blink(u16, Duration, Duration),
    On(u16),
    Off,
}

impl LedStateAnimation {
    pub fn sweep(leds: &[LedState], on_time: Duration, delay: Duration) {
        let mask = Self::led_states_to_mask(leds);
        let _ = CHANNEL.try_send(LedStateAnimation::Sweep(mask, on_time, delay));
    }

    pub fn sweep_long(leds: &[LedState]) {
        Self::sweep(leds, LONG, LONG);
    }

    pub fn sweep_short(leds: &[LedState]) {
        Self::sweep(leds, SHORT, SHORT);
    }

    pub fn sweep_indices(indices: u16, on_time: Duration, delay: Duration) {
        let _ = CHANNEL.try_send(LedStateAnimation::Sweep(indices, on_time, delay));
    }

    pub fn blink(leds: &[LedState], on_time: Duration, delay: Duration) {
        let mask = Self::led_states_to_mask(leds);
        let _ = CHANNEL.try_send(LedStateAnimation::Blink(mask, on_time, delay));
    }

    pub fn blink_short(leds: &[LedState]) {
        Self::blink(leds, SHORT, SHORT);
    }
    pub fn blink_long(leds: &[LedState]) {
        Self::blink(leds, LONG, LONG);
    }

    fn led_states_to_mask(leds: &[LedState]) -> u16 {
        let mut led_indices = 0u16;
        for led in leds {
            led_indices |= 1 << *led as usize;
        }
        led_indices
    }
}

#[embassy_executor::task]
pub async fn led_animation_task() {
    while let state = CHANNEL.recv().await {
        let mut leds = LED.lock().await;
        match state {
            LedStateAnimation::Sweep(indices, on, off) => {
                for state in index_mask_to_enum_iter(indices) {
                    leds.blink(state, on, off).await;
                }
            }
            LedStateAnimation::Blink(indices, on, off) => {
                for state in index_mask_to_enum_iter(indices) {
                    leds.set_state(state);
                }
                Timer::after(on).await;
                leds.set_state(LedState::Off);
                Timer::after(off).await;
            }
            LedStateAnimation::On(indices) => {
                for state in index_mask_to_enum_iter(indices) {
                    leds.set_state(state);
                }
            }
            LedStateAnimation::Off => {
                leds.set_state(LedState::Off);
            }
        }
    }
}


fn index_mask_to_enum_iter(mask: u16) -> impl Iterator<Item=LedState> {
    (0..(LedState::Off as u8)).filter_map(move |index| {
        if mask & (1 << index) == 0 {
            None
        } else {
            Some(LedState::from(index))
        }
    })
}