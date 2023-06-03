use crate::common::device::epd::traits::Command;
use crate::common::device::error::CustomSpimError;

pub trait DisplayInterface {
    async fn cmd<T: Command>(&mut self, command: T) -> Result<(), CustomSpimError>;
    async fn data(&mut self, data: &[u8]) -> Result<(), CustomSpimError>;
    async fn cmd_with_data<T: Command>(
        &mut self,
        command: T,
        data: &[u8],
    ) -> Result<(), CustomSpimError>;
    async fn data_x_times(
        &mut self,
        val: u8,
        repetitions: u32,
    ) -> Result<(), CustomSpimError>;
    async fn write(&mut self, data: &[u8]) -> Result<(), CustomSpimError>;
    async fn wait_until_idle(&mut self, is_busy_low: bool);
    async fn wait_until_idle_with_cmd<T: Command>(
        &mut self,
        is_busy_low: bool,
        status_command: T,
    ) -> Result<(), CustomSpimError>;
    async fn is_busy(&self, is_busy_low: bool) -> bool;
    async fn reset(&mut self, initial_delay: u32, duration: u32);
}
