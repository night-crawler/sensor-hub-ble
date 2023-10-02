use alloc::collections::BTreeMap;

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant};

use crate::common::util::condition::{Condition, ConditionToken};

pub(crate) struct TimeoutTracker<K> {
    instances: Mutex<ThreadModeRawMutex, BTreeMap<K, Instant>>,
    max_duration: Duration,
    condition: Condition<1>,
}

impl<K> TimeoutTracker<K> where K: Ord {
    pub(crate) const fn new(max_duration: Duration) -> Self {
        Self {
            instances: Mutex::new(BTreeMap::new()),
            max_duration,
            condition: Condition::new(),
        }
    }
    pub(crate) async fn register(&self, key: K) {
        let mut instances = self.instances.lock().await;
        if instances.contains_key(&key) {
            return;
        }
        instances.insert(key, Instant::now());

        self.condition.set(!instances.is_empty())
    }

    pub(crate) async fn verify_timeout(&self, key: &K) -> bool {
        let mut instances = self.instances.lock().await;
        if let Some(ts) = instances.get(key) {
            if ts.elapsed() > self.max_duration {
                instances.remove(key);
                return true;
            }
        }
        self.condition.set(!instances.is_empty());
        false
    }

    pub(crate) async fn stop_tracking(&self, key: &K) {
        let mut instances = self.instances.lock().await;
        instances.remove(key);
        self.condition.set(!instances.is_empty());
    }

    pub(crate) async fn wait(&self) -> ConditionToken<1> {
        self.condition.lock().await
    }
}
