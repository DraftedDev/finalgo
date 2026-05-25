use std::time::Duration;

pub const CANDLE_LOOK_BACK: usize = 150;
pub const FETCH_BUFFER: usize = 100;
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(16);
pub const FETCH_RETRIES: usize = 5;
