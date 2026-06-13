use crate::engine::Context;
use crate::indicator::Indicator;
use crate::math::{mean, std_dev};
use std::any::Any;

/// # Bollinger Bands Indicator
///
/// Measures price location and volatility relative to a moving average.
pub struct BollingerBands<const PERIOD: usize, const STD_MULTI: i32> {
    /// Middle Bollinger Band.
    ///
    /// Simple moving average (SMA) of closing price over the configured period.
    ///
    /// Serves as the center line of the Bollinger Band channel and acts as the
    /// baseline reference for trend and mean reversion.
    pub middle: Vec<f64>,

    /// Upper Bollinger Band.
    ///
    /// Computed as:
    ///
    /// ```text
    /// middle + (standard_deviation * STD_MULTI)
    /// ```
    ///
    /// Represents the upper volatility boundary of the channel.
    ///
    /// Prices near or above this band indicate strong upward movement relative
    /// to the recent average.
    pub upper: Vec<f64>,

    /// Lower Bollinger Band.
    ///
    /// Computed as:
    ///
    /// ```text
    /// middle - (standard_deviation * STD_MULTI)
    /// ```
    ///
    /// Represents the lower volatility boundary of the channel.
    ///
    /// Prices near or below this band indicate strong downward movement relative
    /// to the recent average.
    pub lower: Vec<f64>,

    /// Relative Bollinger Band width.
    ///
    /// Computed as:
    ///
    /// ```text
    /// (upper - lower) / middle
    /// ```
    ///
    /// Measures the width of the volatility envelope relative to the price level.
    ///
    /// Larger values indicate volatility expansion.
    /// Smaller values indicate volatility compression.
    pub width: Vec<f64>,

    /// Normalized price position within the Bollinger channel.
    ///
    /// Computed as:
    ///
    /// ```text
    /// (close - lower) / (upper - lower)
    /// ```
    ///
    /// Interprets price location inside the band structure:
    ///
    /// - `0.0` = at the lower band
    /// - `0.5` = near the middle of the channel
    /// - `1.0` = at the upper band
    ///
    /// Values outside `[0, 1]` indicate price trading beyond the bands.
    pub position: Vec<f64>,
}

impl<const PERIOD: usize, const STD_MULTI: i32> BollingerBands<PERIOD, STD_MULTI> {
    pub fn new() -> Self {
        Self {
            middle: Vec::new(),
            upper: Vec::new(),
            lower: Vec::new(),
            width: Vec::new(),
            position: Vec::new(),
        }
    }
}

impl<const PERIOD: usize, const STD_MULTI: i32> Indicator for BollingerBands<PERIOD, STD_MULTI> {
    fn name() -> String {
        format!("bollinger-{}-{}", PERIOD, STD_MULTI)
    }

    fn compute(&mut self, ctx: Context) {
        let closes = &ctx.data().closes;
        let len = closes.len();

        self.middle.resize(len, f64::NAN);
        self.upper.resize(len, f64::NAN);
        self.lower.resize(len, f64::NAN);
        self.width.resize(len, f64::NAN);
        self.position.resize(len, f64::NAN);

        let k = STD_MULTI as f64;

        for idx in (PERIOD - 1)..len {
            let window = &closes[idx + 1 - PERIOD..=idx];

            // safety: skip invalid data instead of poisoning the whole chain
            if window.iter().any(|v| !v.is_finite()) {
                continue;
            }

            let m = mean(window);
            let s = std_dev(window, m);

            if !m.is_finite() || !s.is_finite() {
                continue;
            }

            let upper = m + k * s;
            let lower = m - k * s;
            let range = upper - lower;

            let close = closes[idx];

            if !close.is_finite() {
                continue;
            }

            self.middle[idx] = m;
            self.upper[idx] = upper;
            self.lower[idx] = lower;

            // width (volatility compression/expansion proxy)
            self.width[idx] = if m.abs() > 1e-12 {
                range / m.abs()
            } else {
                0.0
            };

            // do NOT clamp -> keep breakout information
            self.position[idx] = if range.abs() > 1e-12 {
                (close - lower) / range
            } else {
                0.5
            };
        }
    }

    fn is_computed(&self) -> bool {
        !self.width.is_empty()
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
