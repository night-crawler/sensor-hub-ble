use core::sync::atomic::{AtomicBool, Ordering};

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

pub(crate) struct Condition {
    channel: Channel<ThreadModeRawMutex, (), 1>,
    is_enabled: AtomicBool,
}

impl Default for Condition {
    fn default() -> Self {
        Self { channel: Channel::new(), is_enabled: AtomicBool::new(false) }
    }
}

pub(crate) struct ConditionToken<'a> {
    condition: &'a Condition,
}

impl<'a> Drop for ConditionToken<'a> {
    fn drop(&mut self) {
        if self.condition.is_enabled.load(Ordering::SeqCst) {
            let _ = self.condition.channel.try_send(());
        }
    }
}

impl Condition {
    pub const fn new() -> Self {
        Self { channel: Channel::new(), is_enabled: AtomicBool::new(false) }
    }

    pub fn set(&self, value: bool) {
        if value { self.enable() } else { self.disable() }
    }

    pub fn enable(&self) {
        match self.is_enabled.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed) {
            Ok(_) => {
                let _ = self.channel.try_send(());
            }
            Err(_) => {}
        }
    }

    pub fn disable(&self) {
        self.is_enabled.store(false, Ordering::SeqCst);
    }

    pub async fn lock(&self) -> ConditionToken {
        self.channel.recv().await;
        ConditionToken { condition: self }
    }
}
