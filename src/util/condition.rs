use core::sync::atomic::{AtomicBool, Ordering};

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

use crate::ble_debug;

pub(crate) struct Condition<const T: usize> {
    channel: Channel<ThreadModeRawMutex, (), T>,
    is_enabled: AtomicBool,
    name: Option<&'static str>,
}

pub(crate) struct ConditionToken<'a, const T: usize> {
    condition: &'a Condition<T>,
}

impl<'a, const T: usize> Drop for ConditionToken<'a, T> {
    fn drop(&mut self) {
        if self.condition.is_enabled.load(Ordering::SeqCst) {
            if let Err(err) = self.condition.channel.try_send(()) {
                let name = self.condition.name.unwrap_or("unnamed");
                ble_debug!("Failed to return condition token for {name}: {:?}", err);
            }
        }
    }
}

impl<const T: usize> Condition<T> {
    pub const fn new(name: Option<&'static str>) -> Self {
        Self { channel: Channel::new(), is_enabled: AtomicBool::new(false), name }
    }

    pub fn set(&self, value: bool) {
        if value { self.enable() } else { self.disable() }
    }

    pub fn enable(&self) {
        if let Ok(_) =
            self.is_enabled.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
        {
            for _ in 0..T {
                let _ = self.channel.try_send(());
            }
        }
    }

    pub fn fire_once(&self) {
        let _ = self.channel.try_send(());
    }

    pub fn disable(&self) {
        self.is_enabled.store(false, Ordering::SeqCst);
    }

    pub async fn lock(&self) -> ConditionToken<T> {
        self.channel.receive().await;
        ConditionToken { condition: self }
    }
}
