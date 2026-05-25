use crate::consts::{FETCH_BUFFER, FETCH_RETRIES, FETCH_TIMEOUT};
use crate::database::Database;
use crate::utils;
use crate::utils::naive_to_offset;
use serde::{Deserialize, Serialize};
use yahoo_finance_api::YahooConnectorBuilder;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct StockData {
    pub highs: Vec<f64>,
    pub lows: Vec<f64>,
    pub opens: Vec<f64>,
    pub closes: Vec<f64>,
    pub volumes: Vec<f64>,
}

impl StockData {
    pub async fn fetch(database: &Database, key: DataKey) -> Self {
        if let Some(data) = database.get(&key) {
            data
        } else {
            let data = Self::fetch_yahoo(&key).await;
            database.set(&key, &data);
            data
        }
    }

    pub async fn fetch_yahoo(key: &DataKey) -> Self {
        let yahoo = YahooConnectorBuilder::new()
            .timeout(FETCH_TIMEOUT)
            .build()
            .expect("Failed to build yahoo connector");

        let end = utils::parse_naive_date(&key.end);

        // OVER-FETCH (buffer for missing data)
        // TODO: test without fetch buffer
        let start = utils::subtract_naive_date(end, key.size + FETCH_BUFFER);

        let start = naive_to_offset(start);
        let end = naive_to_offset(end);

        let mut response = yahoo.get_quote_history(&key.ticker, start, end).await;
        let mut retries = 1;

        while response.is_err() && retries < FETCH_RETRIES {
            retries += 1;
            tracing::warn!("Fetch failed. Retrying ({retries}/{FETCH_RETRIES})...");
            response = yahoo.get_quote_history(&key.ticker, start, end).await;
        }

        let mut quotes = response
            .expect("Failed to fetch quotes")
            .quotes()
            .expect("Failed to get quotes");

        quotes.sort_by_key(|c| c.timestamp);

        let len = quotes.len();
        let start_idx = len.saturating_sub(key.size);

        let trimmed = &quotes[start_idx..];

        Self {
            highs: trimmed.iter().map(|q| q.high).collect(),
            lows: trimmed.iter().map(|q| q.low).collect(),
            opens: trimmed.iter().map(|q| q.open).collect(),
            closes: trimmed.iter().map(|q| q.close).collect(),
            volumes: trimmed.iter().map(|q| q.volume as f64).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DataKey {
    pub size: usize,
    pub end: String,
    pub ticker: String,
}
