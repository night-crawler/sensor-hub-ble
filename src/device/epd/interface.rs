use crate::common::device::epd::traits::Command;
use crate::common::device::error::CustomSpimError;

pub trait DisplayInterface {
    async fn send_command<T: Command>(&mut self, command: T) -> Result<(), CustomSpimError>;
    async fn send_data(&mut self, data: &[u8]) -> Result<(), CustomSpimError>;
    async fn send_command_with_data<T: Command>(
        &mut self,
        command: T,
        data: &[u8],
    ) -> Result<(), CustomSpimError>;
    async fn send_data_x_times(
        &mut self,
        val: u8,
        repetitions: u32,
    ) -> Result<(), CustomSpimError>;
    async fn write(&mut self, data: &[u8]) -> Result<(), CustomSpimError>;
    async fn wait_until_idle(&mut self, is_busy_low: bool);
    async fn reset(&mut self, initial_delay: u32, duration: u32);
}
