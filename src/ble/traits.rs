pub(crate) trait IsTaskEnabled {
    fn is_task_enabled(&self) -> bool;
}

pub(crate) trait SettingsEventConsumer<E> {
    fn consume(&mut self, event: E);
}

pub(crate) trait TimeoutEventCharacteristic {
    fn get_timeout(&self) -> Option<u32>;
}
