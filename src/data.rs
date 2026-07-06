use crate::consts::FETCH_RETRIES;
use crate::utils;
use crate::utils::FastMap;
use chrono::Datelike;
use ibapi::Client;
use ibapi::contracts::Contract;
use ibapi::market_data::historical;
use ibapi::market_data::historical::{Bar, BarSize, BarTimestamp, WhatToShow};
use std::time::Duration;

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
    /// Fetches the stock data from the Interactive Brokers TWS/IB Gateway API.
    pub async fn fetch(client: &Client, key: &DataKey) -> Self {
        let end_date = utils::parse_naive_date(&key.end);

        let end_time_date = time::Date::from_calendar_date(
            end_date.year(),
            time::Month::try_from(end_date.month() as u8).unwrap(),
            end_date.day() as u8,
        )
        .unwrap();

        let end_time = end_time_date.with_hms(23, 59, 59).unwrap().assume_utc();

        let mut retries = 0;

        let bars = loop {
            let contract = Contract::stock(&key.ticker)
                .on_exchange("SMART")
                .in_currency("USD")
                .build();

            let res = client
                .historical_data(&contract, BarSize::Day)
                .what_to_show(WhatToShow::Trades)
                .ending(end_time)
                .duration(historical::Duration::days(key.size as i32))
                .fetch()
                .await;

            match res {
                Ok(data) => break data.bars,
                Err(e) => {
                    if retries < FETCH_RETRIES {
                        tracing::warn!("IBKR fetch failed: {e}");
                        retries += 1;
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    } else {
                        panic!("Failed to fetch from IBKR after maximum retries: {e}");
                    }
                }
            }
        };

        if bars.is_empty() {
            panic!("IBKR returned 0 bars for {}.", key.ticker);
        }

        let last_bar = bars.last().unwrap();

        let bar_date = match last_bar.date {
            BarTimestamp::Date(d) => d,
            BarTimestamp::DateTime(dt) => dt.date(),
        };

        let bar_date_str = format!(
            "{:02}.{:02}.{}",
            bar_date.day(),
            u8::from(bar_date.month()),
            bar_date.year()
        );

        if bar_date_str != key.end {
            panic!(
                "Date mismatch for {}: requested {}, but latest candle is from {}.",
                key.ticker, key.end, bar_date_str
            );
        }

        Self::from_bar(bars)
    }

    /// Converts the IBKR Bar vector into our internal StockData structure.
    fn from_bar(bars: Vec<Bar>) -> Self {
        Self {
            opens: bars.iter().map(|b| b.open).collect(),
            highs: bars.iter().map(|b| b.high).collect(),
            lows: bars.iter().map(|b| b.low).collect(),
            closes: bars.iter().map(|b| b.close).collect(),
            volumes: bars.iter().map(|b| b.volume).collect(),
        }
    }
}

/// A key used to identify stock data.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct DataKey {
    pub size: usize,
    pub end: String,
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

        let start_time_date = time::Date::from_calendar_date(
            start_date.year(),
            time::Month::try_from(start_date.month() as u8).unwrap(),
            start_date.day() as u8,
        )
        .unwrap();

        let end_time_date = time::Date::from_calendar_date(
            end_date.year(),
            time::Month::try_from(end_date.month() as u8).unwrap(),
            end_date.day() as u8,
        )
        .unwrap();

        let mut current_end = end_time_date.with_hms(23, 59, 59).unwrap().assume_utc();
        let start_time = start_time_date.with_hms(0, 0, 0).unwrap().assume_utc();

        let contract = Contract::stock(&ticker)
            .on_exchange("SMART")
            .in_currency("USD")
            .build();

        // Collect chunks in a separate vector to preserve chronological order
        let mut chunks = Vec::new();

        while current_end > start_time {
            let mut chunk_retries = 0;

            let bars = loop {
                let res = client
                    .historical_data(&contract, BarSize::Day)
                    .what_to_show(WhatToShow::Trades)
                    .ending(current_end)
                    .duration(historical::Duration::YEAR)
                    .fetch()
                    .await;

                match res {
                    Ok(data) => break data.bars,
                    Err(e) => {
                        if chunk_retries < FETCH_RETRIES {
                            tracing::warn!("IBKR chunk fetch failed for {}: {e}", ticker);
                            chunk_retries += 1;
                            tokio::time::sleep(Duration::from_secs(15)).await;
                        } else {
                            panic!(
                                "Failed to fetch chunk from IBKR for {} after max retries: {e}",
                                ticker
                            );
                        }
                    }
                }
            };

            if bars.is_empty() {
                tracing::warn!(
                    "IBKR returned 0 bars for chunk ending at {:?}. Stopping early.",
                    current_end
                );
                break;
            }

            let oldest_bar_date = match &bars.first().unwrap().date {
                BarTimestamp::Date(d) => *d,
                BarTimestamp::DateTime(dt) => dt.date(),
            };

            chunks.push(bars);

            match oldest_bar_date.previous_day() {
                Some(prev_day) => {
                    current_end = prev_day.with_hms(23, 59, 59).unwrap().assume_utc();
                }
                None => break,
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        // Reverse newest-to-oldest order
        let mut all_bars = Vec::new();
        for chunk in chunks.into_iter().rev() {
            all_bars.extend(chunk);
        }

        tracing::info!("Cached {} bars for {}", all_bars.len(), ticker);
        self.bars.insert(ticker, all_bars);
    }

    /// Slices the cached bars in memory to match the exact [DataKey] window.
    pub fn get_stock_data(&self, key: &DataKey) -> Option<StockData> {
        let bars = self.bars.get(&key.ticker)?;
        let end_date = utils::parse_naive_date(&key.end);

        let end_date_time = time::Date::from_calendar_date(
            end_date.year(),
            time::Month::try_from(end_date.month() as u8).unwrap(),
            end_date.day() as u8,
        )
        .unwrap();

        let mut end_idx = None;

        for (i, bar) in bars.iter().enumerate().rev() {
            let bar_date = match bar.date {
                BarTimestamp::Date(d) => d,
                BarTimestamp::DateTime(dt) => dt.date(),
            };

            if bar_date <= end_date_time {
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
