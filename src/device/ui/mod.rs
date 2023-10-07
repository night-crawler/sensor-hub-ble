use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use lazy_static::lazy_static;

use crate::common::device::ui::ui_store::UiStore;
use crate::common::device::ui::controls::{ButtonState, DisplayRefreshType};
use embassy_sync::channel::Channel;

pub(crate) mod device_ui;

pub(crate) mod ui_store;
pub(crate) mod error;

#[macro_use]
pub(crate) mod ui_macro;
pub(crate) mod text_repr;
pub(crate) mod controls;

lazy_static! {
    pub(crate) static ref UI_STORE: Mutex<ThreadModeRawMutex, UiStore> = Mutex::new(UiStore::default());
    pub(crate) static ref BUTTON_STATE: Mutex<ThreadModeRawMutex, ButtonState> = Mutex::new(ButtonState::default());
}
pub(crate) static BUTTON_EVENTS: Channel<
    ThreadModeRawMutex,
    ButtonState,
    5,
> = Channel::new();
pub(crate) static DISPLAY_REFRESH_EVENTS: Channel<
    ThreadModeRawMutex,
    DisplayRefreshType,
    1,
> = Channel::new();
