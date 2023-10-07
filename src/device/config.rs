use embassy_time::Duration;

pub(crate) const NUM_CONNECTIONS: usize = 3;

pub(crate) const BLE_DEBUG_QUEUE_LEN: usize = 2;
pub(crate) const BLE_DEBUG_ARRAY_LEN: usize = 128;

pub(crate) const BLE_EXPANDER_CONTROL_BYTES_SIZE: usize = 16;
pub(crate) const BLE_EXPANDER_BUF_SIZE: usize = 512 - BLE_EXPANDER_CONTROL_BYTES_SIZE;
pub(crate) const BLE_EXPANDER_LOCK_TIMEOUT: Duration = Duration::from_secs(20);
pub(crate) const BLE_EXPANDER_EXEC_TIMEOUT: Duration = Duration::from_millis(2000);

pub(crate) const DEBOUNCE_INTERVAL: Duration = Duration::from_millis(50);

// Color sensor oversampling takes a lot of time
pub(crate) const ALL_TASK_COMPLETION_INTERVAL: Duration = Duration::from_millis(3000);

pub(crate) const FLASH_PAGE_SIZE: usize = 4096;
pub(crate) const CONFIG_FLASH_SIZE: usize = FLASH_PAGE_SIZE - 4;
pub(crate) const INIT_TOKEN: [u8; 4] = [0xBB, 0x3D, 0x12, 0x3A];
