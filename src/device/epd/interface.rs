use crate::common::device::epd::traits::Command;

pub trait DisplayInterface<E> {
    async fn send_command<T: Command>(&mut self, command: T) -> Result<(), E>;
    async fn send_data(&mut self, data: &[u8]) -> Result<(), E>;
    async fn send_command_with_data<T: Command>(
        &mut self,
        command: T,
        data: &[u8],
    ) -> Result<(), E>;
    async fn send_data_x_times(
        &mut self,
        val: u8,
        repetitions: u32,
    ) -> Result<(), E>;
    async fn write(&mut self, data: &[u8]) -> Result<(), E>;
    async fn wait_until_idle(&mut self, is_busy_low: bool);
    async fn reset(&mut self, initial_delay: u32, duration: u32);
}
