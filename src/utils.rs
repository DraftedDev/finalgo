use indicatif::ProgressStyle;
use std::sync::LazyLock;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::{Date, UtcOffset};
use tracing_indicatif::span_ext::IndicatifSpanExt;
use trading_calendar::{Market, NaiveDate, TradingCalendar};
use yahoo_finance_api::time::OffsetDateTime;

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

pub fn assert_range(value: f64, min: f64, max: f64, label: &str) {
    assert!(value.is_finite(), "{label} value must be finite");

    assert!(
        value >= min && value <= max,
        "{label} value {value} out of bounds [{min}, {max}]"
    );
}

pub async fn with_progress<Fut: Future<Output = R>, R>(
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
