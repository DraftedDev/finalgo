use crate::math;
use apca::{ApiInfo, Client};
use indicatif::ProgressStyle;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::LazyLock;
use tracing_indicatif::span_ext::IndicatifSpanExt;
use trading_calendar::{Market, NaiveDate, TradingCalendar};

/// [HashMap] with the [rustc_hash::FxBuildHasher] for maximum performance.
pub type FastMap<K, V> = HashMap<K, V, rustc_hash::FxBuildHasher>;

static CALENDAR: LazyLock<TradingCalendar> = LazyLock::new(|| {
    TradingCalendar::new(Market::NASDAQ).expect("Failed to build trading calendar")
});

const CHRONO_FORMAT: &str = "%d.%m.%Y";

/// Parses a date string into a [NaiveDate] using the format `dd.mm.yyyy`.
pub fn parse_naive_date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, CHRONO_FORMAT).expect("Failed to parse date")
}

/// Formats a [NaiveDate] into a string using the format `dd.mm.yyyy`.
pub fn format_naive_date(date: NaiveDate) -> String {
    date.format(CHRONO_FORMAT).to_string()
}

/// Subtracts a number of trading days from a [NaiveDate].
pub fn subtract_naive_date(date: NaiveDate, count: usize) -> NaiveDate {
    let mut result = date;

    for _ in 0..count {
        result = CALENDAR.previous_trading_day(result);
    }

    result
}

/// Adds a number of trading days to a [NaiveDate].
pub fn add_naive_date(date: NaiveDate, count: usize) -> NaiveDate {
    let mut result = date;

    for _ in 0..count {
        result = CALENDAR.next_trading_day(result);
    }

    result
}

/// Runs a function with a progress bar in order to display progress to the end user.
///
/// The bar can be progressed by calling [tracing::Span::pb_inc] or similar methods.
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

/// Runs an asynchronous function with a progress bar in order to display progress to the end user.
///
/// The bar can be progressed by calling [tracing::Span::pb_inc] or similar methods.
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

pub fn client() -> Client {
    Client::new(
        ApiInfo::from_parts(
            "https://data.alpaca.markets/",
            read_secret("ALPACA_KEY"),
            read_secret("ALPACA_SECRET"),
        )
        .expect("Failed to build Alpaca Client"),
    )
}

pub fn read_secret(name: &str) -> String {
    let path = Path::new("secrets").join(name);

    if path.exists() {
        std::fs::read_to_string(path)
            .expect("Failed to read secret")
            .replace(|c: char| c.is_whitespace() || c == '\r' || c == '\n', "")
    } else {
        std::fs::write(&path, "").expect("Failed to write secret file");

        panic!(
            "Secret '{}' not found. Write the secret to {} please.",
            name,
            path.display()
        );
    }
}

/// A [FastMap] of [Value]s.
///
/// Used for communications between scores and the metrics system.
///
/// Inserted values are ordered by insertion order.
///
/// The [Display] implementation of this structure will print the values in order.
pub struct ValueMap {
    fields: FastMap<String, Value>,
    order: Vec<String>,
}

impl ValueMap {
    /// Creates a new [ValueMap].
    pub fn new() -> Self {
        Self {
            fields: FastMap::with_capacity_and_hasher(16, Default::default()),
            order: Vec::new(),
        }
    }

    /// Add a field to the [ValueMap].
    ///
    /// Panics if the field already exists.
    pub fn add(&mut self, key: impl ToString, field: impl Into<Value>) {
        let key = key.to_string();

        if self.fields.contains_key(&key) {
            panic!("Field already set");
        }

        self.order.push(key.clone());
        self.fields.insert(key, field.into());
    }

    /// Add a field to the [ValueMap] and return the [ValueMap] itself.
    ///
    /// See [ValueMap::add] for more.
    pub fn with(mut self, key: impl ToString, field: impl Into<Value>) -> Self {
        self.add(key, field);
        self
    }

    /// Merges another [ValueMap] into this one.
    ///
    /// Panics if a field already exists.
    pub fn merge(&mut self, mut other: ValueMap) {
        for key in other.order {
            let value = other
                .fields
                .remove(&key)
                .expect("Failed to get field value");
            self.add(key, value);
        }
    }
}

impl Display for ValueMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut fields = Vec::with_capacity(self.order.len());

        for key in &self.order {
            fields.push((
                key,
                self.fields.get(key).expect("Failed to get field value"),
            ));
        }

        for (key, value) in fields {
            writeln!(f, "\t{key} : [ {value} ]")?;
        }

        Ok(())
    }
}

/// A Value that can represent multiple native values.
#[derive(Clone, Debug)]
pub enum Value {
    /// A float/double value.
    ///
    /// The [Display] implementation rounds to 2 decimal places.
    Float(f64),

    /// A percentage value.
    ///
    /// The [Display] implementation rounds to 2 decimal places with a '%' character at the end.
    Percent(f64),

    /// An integer value.
    Int(i64),

    /// A string value.
    String(String),
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Float(fl) => write!(f, "{}", math::round_to(*fl, 2)),
            Value::Percent(p) => write!(f, "{} %", math::round_to(*p * 100.0, 2)),
            Value::Int(i) => write!(f, "{}", i),
            Value::String(s) => write!(f, "{}", s),
        }
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Float(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Int(value)
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
