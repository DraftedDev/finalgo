use crate::consts::{FETCH_RETRIES, FETCH_TIMEOUT};
use crate::database::Database;
use crate::utils;
use crate::utils::naive_to_offset;
use serde::{Deserialize, Serialize};
use yahoo_finance_api::YahooConnectorBuilder;

/// The fetched stock data value with highs, lows, opens, closes, and volumes.
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct StockData {
    pub highs: Vec<f64>,
    pub lows: Vec<f64>,
    pub opens: Vec<f64>,
    pub closes: Vec<f64>,
    pub volumes: Vec<f64>,
}

impl StockData {
    /// Fetches the stock data.
    ///
    /// If the data is not found in the database, it will be fetched from the Yahoo Finance API.
    /// This process can take longer if the data must be fetched from the API.
    pub async fn fetch(database: &mut Database, key: DataKey) -> Self {
        if let Some(data) = database.get(&key) {
            data
        } else {
            tracing::info!("StockData not found in database. Fetching from Yahoo...");
            let data = Self::fetch_yahoo(&key).await;
            database.set(key, data.clone());
            data
        }
    }

    /// Fetches the stock data from the Yahoo Finance API.
    pub async fn fetch_yahoo(key: &DataKey) -> Self {
        let yahoo = YahooConnectorBuilder::new()
            .timeout(FETCH_TIMEOUT)
            .build()
            .expect("Failed to build yahoo connector");

        let end = utils::parse_naive_date(&key.end);
        let start = utils::subtract_naive_date(end, key.size);

        let start = naive_to_offset(start);

        let api_end = utils::add_naive_date(end, 1);
        let end = naive_to_offset(api_end);

        let mut response = yahoo.get_quote_history(&key.ticker, start, end).await;
        let mut retries = 1;

        while response.is_err() && retries < FETCH_RETRIES {
            tracing::warn!("Fetch failed. Retrying ({retries}/{FETCH_RETRIES})...");
            response = yahoo.get_quote_history(&key.ticker, start, end).await;
            retries += 1;
        }

        let quotes = response
            .expect("Failed to fetch quotes")
            .quotes()
            .expect("Failed to get quotes");

        if quotes.is_empty() {
            panic!(
                "Yahoo Finance returned 0 candles for {} up to {}. \
                The date is likely in the future, or the ticker is invalid.",
                key.ticker, key.end
            );
        }

        let last_quote = quotes.last().unwrap();
        let last_dt = time::OffsetDateTime::from_unix_timestamp(last_quote.timestamp)
            .expect("Invalid timestamp from Yahoo Finance");

        let last_date_str = format!(
            "{:02}.{:02}.{}",
            last_dt.day(),
            last_dt.month() as u8,
            last_dt.year()
        );

        if last_date_str != key.end {
            panic!(
                "Requested data for {} ending on {}, but the latest available candle is from {}.",
                key.ticker, key.end, last_date_str
            );
        }

        Self {
            highs: quotes.iter().map(|q| q.high).collect(),
            lows: quotes.iter().map(|q| q.low).collect(),
            opens: quotes.iter().map(|q| q.open).collect(),
            closes: quotes.iter().map(|q| q.close).collect(),
            volumes: quotes.iter().map(|q| q.volume as f64).collect(),
        }
    }
}

/// A key used to identify stock data.
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DataKey {
    /// The size of the associated [StockData].
    pub size: usize,
    /// The end date of the associated [StockData].
    pub end: String,
    /// The ticker of the associated [StockData].
    pub ticker: String,
}
