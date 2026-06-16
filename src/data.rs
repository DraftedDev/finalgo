use crate::consts::FETCH_RETRIES;
use crate::database::Database;
use crate::utils;
use apca::Client;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use trading_calendar::{NaiveDate, Utc};

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
    /// If the data is not found in the database, it will be fetched from the Alpaca Finance API.
    /// This process can take longer if the data must be fetched from the API.
    pub async fn fetch(database: &mut Database, client: &Client, key: DataKey) -> Self {
        if let Some(data) = database.get(&key) {
            data
        } else {
            tracing::info!("StockData not found in database. Fetching from Alpaca...");
            let data = Self::fetch_alpaca(client, &key).await;
            database.set(key, data.clone());
            data
        }
    }

    /// Fetches the stock data from the Alpaca Finance API.
    pub async fn fetch_alpaca(client: &Client, key: &DataKey) -> Self {
        let end_date = utils::parse_naive_date(&key.end);
        let start_date = utils::subtract_naive_date(end_date, key.size);
        let api_end_date = utils::add_naive_date(end_date, 1);

        let start_chrono =
            NaiveDate::from_ymd_opt(start_date.year(), start_date.month(), start_date.day())
                .expect("Invalid start date")
                .and_hms_opt(0, 0, 0)
                .expect("Invalid start time")
                .and_local_timezone(Utc)
                .unwrap();

        let end_chrono = NaiveDate::from_ymd_opt(
            api_end_date.year(),
            api_end_date.month(),
            api_end_date.day(),
        )
        .expect("Invalid end date")
        .and_hms_opt(0, 0, 0)
        .expect("Invalid end time")
        .and_local_timezone(Utc)
        .unwrap();

        let request = apca::data::v2::bars::ListReqInit {
            limit: None,
            adjustment: Some(apca::data::v2::bars::Adjustment::Raw),
            feed: None,
            page_token: None,
            _non_exhaustive: (),
        }
        .init(
            key.ticker.clone(),
            start_chrono,
            end_chrono,
            apca::data::v2::bars::TimeFrame::OneDay,
        );

        let mut response = client.issue::<apca::data::v2::bars::List>(&request).await;

        let mut retries = 1;

        while response.is_err() && retries < FETCH_RETRIES {
            tracing::warn!("Alpaca fetch failed. Retrying ({retries}/{FETCH_RETRIES})...");
            response = client.issue::<apca::data::v2::bars::List>(&request).await;
            retries += 1;
        }

        let bars_response = response.expect("Failed to fetch from Alpaca after maximum retries");
        let bars = bars_response.bars;

        if bars.is_empty() {
            panic!("Alpaca returned 0 bars for {}.", key.ticker);
        }

        let last_bar = bars.last().unwrap();

        let naive_date = last_bar.time.date_naive();
        let last_date_str = format!(
            "{:02}.{:02}.{}",
            naive_date.day(),
            naive_date.month(),
            naive_date.year()
        );

        if last_date_str != key.end {
            tracing::warn!(
                "Date mismatch for {}: requested {}, but latest candle is from {}. Using available data.",
                key.ticker,
                key.end,
                last_date_str
            );
        }

        Self {
            opens: bars
                .iter()
                .map(|b| {
                    b.open
                        .to_string()
                        .parse::<f64>()
                        .expect("Failed to parse open")
                })
                .collect(),
            highs: bars
                .iter()
                .map(|b| {
                    b.high
                        .to_string()
                        .parse::<f64>()
                        .expect("Failed to parse high")
                })
                .collect(),
            lows: bars
                .iter()
                .map(|b| {
                    b.low
                        .to_string()
                        .parse::<f64>()
                        .expect("Failed to parse low")
                })
                .collect(),
            closes: bars
                .iter()
                .map(|b| {
                    b.close
                        .to_string()
                        .parse::<f64>()
                        .expect("Failed to parse close")
                })
                .collect(),
            volumes: bars.iter().map(|b| b.volume as f64).collect(),
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
