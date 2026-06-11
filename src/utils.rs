use crate::utils;
use indicatif::ProgressStyle;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::sync::LazyLock;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::{Date, UtcOffset};
use tracing_indicatif::span_ext::IndicatifSpanExt;
use trading_calendar::{Market, NaiveDate, TradingCalendar};
use yahoo_finance_api::time::OffsetDateTime;

pub type FastMap<K, V> = HashMap<K, V, rustc_hash::FxBuildHasher>;

static CALENDAR: LazyLock<TradingCalendar> = LazyLock::new(|| {
    TradingCalendar::new(Market::NASDAQ).expect("Failed to build trading calendar")
});

const CHRONO_FORMAT: &str = "%d.%m.%Y";
const TIME_FORMAT: &[BorrowedFormatItem] = format_description!("[day].[month].[year]");

pub fn parse_naive_date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, CHRONO_FORMAT).expect("Failed to parse date")
}

pub fn format_naive_date(date: NaiveDate) -> String {
    date.format(CHRONO_FORMAT).to_string()
}

pub fn naive_to_offset(date: NaiveDate) -> OffsetDateTime {
    let fmt = date.format(CHRONO_FORMAT).to_string();

    Date::parse(&fmt, &TIME_FORMAT)
        .expect("Failed to parse date")
        .midnight()
        .assume_offset(UtcOffset::from_hms(2, 0, 0).expect("Failed to create offset"))
}

pub fn subtract_naive_date(date: NaiveDate, count: usize) -> NaiveDate {
    let mut result = date;

    for _ in 0..count {
        result = CALENDAR.previous_trading_day(result);
    }

    result
}

pub fn add_naive_date(date: NaiveDate, count: usize) -> NaiveDate {
    let mut result = date;

    for _ in 0..count {
        result = CALENDAR.next_trading_day(result);
    }

    result
}

pub fn round_to_two_decimals(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

pub fn with_progress<R>(msg: &str, len: u64, f: impl FnOnce(tracing::Span) -> R) -> R {
    let span = tracing::span!(tracing::Level::INFO, "progress");
    span.pb_set_message(msg);
    span.pb_set_length(len);

    let template = if len == 0 {
        "  [{spinner:.green}] {msg} │ {elapsed:<4}"
    } else {
        "  [{spinner:.green}] {msg} {wide_bar:.green/red} {pos}/{len} ({percent}%) │ {elapsed:<4}"
    };

    span.pb_set_style(
        &ProgressStyle::with_template(template)
            .unwrap()
            .progress_chars("━━━"),
    );

    let span2 = span.clone();
    let enter = span2.enter();
    let result = f(span);

    drop(enter);

    result
}

pub async fn with_progress_async<Fut: Future<Output = R>, R>(
    msg: &str,
    len: u64,
    f: impl FnOnce(tracing::Span) -> Fut,
) -> R {
    let span = tracing::span!(tracing::Level::INFO, "progress");
    span.pb_set_message(msg);
    span.pb_set_length(len);

    let template = if len == 0 {
        "  [{spinner:.green}] {msg} │ {elapsed:<4}"
    } else {
        "  [{spinner:.green}] {msg} {wide_bar:.green/red} {pos}/{len} ({percent}%) │ {elapsed:<4}"
    };

    span.pb_set_style(
        &ProgressStyle::with_template(template)
            .unwrap()
            .progress_chars("━━━"),
    );

    let span2 = span.clone();
    let enter = span2.enter();
    let result = f(span).await;

    drop(enter);

    result
}

pub struct ValueMap {
    fields: FastMap<String, Value>,
}

impl ValueMap {
    pub fn new() -> Self {
        Self {
            fields: FastMap::with_capacity_and_hasher(16, Default::default()),
        }
    }

    pub fn add(&mut self, key: impl ToString, field: impl Into<Value>) {
        let key = key.to_string();

        if self.fields.contains_key(&key) {
            panic!("Field already set");
        }

        self.fields.insert(key, field.into());
    }

    pub fn get(&self, key: &str) -> &Value {
        self.fields.get(key).expect("Failed to get field value")
    }

    pub fn merge(&mut self, other: ValueMap) {
        for (k, v) in other.fields {
            self.add(k, v);
        }
    }
}

impl Display for ValueMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (key, value) in &self.fields {
            writeln!(f, "\t{key} : [ {value} ]")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Num(f64),
    String(String),
}

impl Value {
    pub fn as_num(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            Value::String(_) => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Num(_) => None,
            Value::String(s) => Some(s.as_str()),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Num(n) => write!(f, "{}", utils::round_to_two_decimals(*n)),
            Value::String(s) => write!(f, "{}", s),
        }
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Num(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.to_string())
    }
}
