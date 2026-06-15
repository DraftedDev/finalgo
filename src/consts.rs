use std::time::Duration;

/// The candle look back period.
///
/// Fetched prediction stock data will have roughly this many candles.
pub const CANDLE_LOOK_BACK: usize = 610;

/// The target horizon of the algorithm.
///
/// Currently equal to 7 days (1 week).
pub const TARGET_HORIZON: usize = 7;

/// The timeout for fetching stock data from Yahoo Finance.
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(16);

/// The number of retries for fetching stock data from Yahoo Finance.
///
/// If fetching fails, due to timeouts or other issues,
/// the request will be retried, but maximally this many times.
pub const FETCH_RETRIES: usize = 5;

/// The number of datasets to fetch in parallel.
///
/// Setting this value too high can lead to rate-limiting from the Yahoo Finance API.
pub const FETCH_CHUNK_SIZE: usize = 10;

/// The size of the database memory map.
pub const DATABASE_MEM_MAP: usize = 1024 * 1024 * 1024 * 16;

/// The target dead zone of the algorithm.
///
/// This is the minimum threshold at which the target direction will not be considered neutral.
pub const TARGET_DEAD_ZONE: f64 = 0.015;
