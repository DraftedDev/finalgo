use std::time::Duration;

pub const CANDLE_LOOK_BACK: usize = 150;
pub const TARGET_CANDLE_LOOK_BACK: usize = 5;
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(16);
pub const FETCH_RETRIES: usize = 5;
pub const FETCH_CHUNK_SIZE: usize = 10;
pub const DATABASE_MEM_MAP: usize = 1024 * 1024 * 64;
