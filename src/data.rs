use crate::consts::FETCH_RETRIES;
use crate::utils::FastMap;
use crate::{consts, utils};
use apca::data::v2::bars::{Bar, ListError};
use apca::{Client, RequestError};
use chrono::Datelike;
use std::time::Duration;
use trading_calendar::{NaiveDate, Utc};

/// The fetched stock data value with highs, lows, opens, closes, and volumes.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct StockData {
    pub highs: Vec<f64>,
    pub lows: Vec<f64>,
    pub opens: Vec<f64>,
    pub closes: Vec<f64>,
    pub volumes: Vec<f64>,
}

impl StockData {
    /// Fetches the stock data from the Alpaca Finance API.
    pub async fn fetch(client: &Client, key: &DataKey) -> Self {
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
            adjustment: Some(apca::data::v2::bars::Adjustment::Split),
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

        while let Err(err) = &response
            && retries < FETCH_RETRIES
        {
            if let RequestError::Endpoint(err) = err
                && let ListError::RateLimitExceeded(_) = err
            {
                tracing::info!(
                    "Rate limit reached. Waiting {}s...",
                    consts::RATE_LIMIT_WAIT
                );

                tokio::time::sleep(Duration::from_secs(consts::RATE_LIMIT_WAIT)).await;
            }

            tracing::warn!("Alpaca fetch failed: {err}");
            tracing::info!("Retrying ({retries}/{FETCH_RETRIES})...");

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
            panic!(
                "Date mismatch for {}: requested {}, but latest candle is from {}.",
                key.ticker, key.end, last_date_str
            );
        }

        Self::from_bar(bars)
    }

    fn from_bar(bars: Vec<Bar>) -> Self {
        Self {
            opens: bars
                .iter()
                .map(|b| b.open.to_f64().expect("Failed to parse open"))
                .collect(),
            highs: bars
                .iter()
                .map(|b| b.high.to_f64().expect("Failed to parse high"))
                .collect(),
            lows: bars
                .iter()
                .map(|b| b.low.to_f64().expect("Failed to parse low"))
                .collect(),
            closes: bars
                .iter()
                .map(|b| b.close.to_f64().expect("Failed to parse close"))
                .collect(),
            volumes: bars.iter().map(|b| b.volume as f64).collect(),
        }
    }
}

/// A key used to identify stock data.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct DataKey {
    /// The size of the associated [StockData].
    pub size: usize,
    /// The end date of the associated [StockData].
    pub end: String,
    /// The ticker of the associated [StockData].
    pub ticker: String,
}

/// Cache that fetches bulk data and slices it in memory to avoid API rate limits.
pub struct DataCache {
    bars: FastMap<String, Vec<Bar>>,
}

impl DataCache {
    pub fn new() -> Self {
        Self {
            bars: FastMap::with_capacity_and_hasher(16, Default::default()),
        }
    }

    /// Fetches the entire date range for a ticker in a single API call and caches it.
    pub async fn fetch_range(
        &mut self,
        client: &Client,
        ticker: String,
        start: String,
        end: String,
    ) {
        let start_date = utils::parse_naive_date(&start);
        let end_date = utils::parse_naive_date(&end);
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
            limit: Some(10000),
            adjustment: Some(apca::data::v2::bars::Adjustment::Split),
            feed: None,
            page_token: None,
            _non_exhaustive: (),
        }
        .init(
            ticker.clone(),
            start_chrono,
            end_chrono,
            apca::data::v2::bars::TimeFrame::OneDay,
        );

        let mut response = client.issue::<apca::data::v2::bars::List>(&request).await;
        let mut retries = 1;

        while let Err(err) = &response
            && retries < FETCH_RETRIES
        {
            if let RequestError::Endpoint(err) = err
                && let ListError::RateLimitExceeded(_) = err
            {
                tracing::info!(
                    "Rate limit reached. Waiting {}s...",
                    consts::RATE_LIMIT_WAIT
                );

                tokio::time::sleep(Duration::from_secs(consts::RATE_LIMIT_WAIT)).await;
            }

            tracing::warn!("Alpaca fetch failed: {err}");
            tracing::info!("Retrying ({retries}/{FETCH_RETRIES})...");

            response = client.issue::<apca::data::v2::bars::List>(&request).await;
            retries += 1;
        }

        let bars_response = response.expect("Failed to fetch bulk data from Alpaca");
        tracing::info!("Cached {} bars for {}", bars_response.bars.len(), ticker);

        self.bars.insert(ticker, bars_response.bars);
    }

    /// Slices the cached bars in memory to match the exact [DataKey] window.
    pub fn get_stock_data(&self, key: &DataKey) -> Option<StockData> {
        let bars = self.bars.get(&key.ticker)?;
        let end_date = utils::parse_naive_date(&key.end);

        let mut end_idx = None;
        for (i, bar) in bars.iter().enumerate().rev() {
            let bar_date = bar.time.date_naive();
            let bar_naive =
                NaiveDate::from_ymd_opt(bar_date.year(), bar_date.month(), bar_date.day()).unwrap();

            if bar_naive <= end_date {
                end_idx = Some(i);
                break;
            }
        }

        let end_idx = end_idx?;

        let start_idx = end_idx.saturating_sub(key.size - 1);
        let sliced_bars = bars[start_idx..=end_idx].to_vec();

        if sliced_bars.len() < key.size {
            return None;
        }

        Some(StockData::from_bar(sliced_bars))
    }
}
