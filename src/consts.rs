/// The candle look back period.
///
/// Fetched prediction stock data will have roughly this many candles.
pub const CANDLE_LOOK_BACK: usize = 610;

/// The target horizon of the algorithm.
///
/// Currently equal to 5 trading days (1 week).
pub const TARGET_HORIZON: usize = 5;

/// The number of retries for fetching stock data from the Alpaca API.
///
/// If fetching fails, due to timeouts or other issues,
/// the request will be retried, but maximally this many times.
pub const FETCH_RETRIES: usize = 5;

/// The number of datasets to fetch in parallel.
///
/// Setting this value too high can lead to rate-limiting from the Alpaca API.
pub const FETCH_CHUNK_SIZE: usize = 10;

/// The target dead zone of the algorithm.
///
/// This is the minimum threshold at which the target direction will not be considered neutral.
pub const TARGET_DEAD_ZONE: f64 = 0.015;
