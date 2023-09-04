use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use lazy_static::lazy_static;

use crate::common::device::ui::ui_store::UiStore;

pub(crate) mod device_ui;

pub(crate) mod ui_store;
pub(crate) mod error;

#[macro_use]
pub(crate) mod ui_macro;
pub(crate) mod text_repr;

lazy_static! {
    pub(crate) static ref UI_STORE: Mutex<ThreadModeRawMutex, UiStore> = Mutex::new(UiStore::default());
}
