use crate::engine::Context;
use crate::indicator::Indicator;
use std::any::Any;

/// # Exponential Moving Average Indicator
///
/// Tracks trend direction and price location relative to a smoothed average.
pub struct ExpMovAvg<const PERIOD: usize> {
    /// Exponential Moving Average (EMA).
    ///
    /// A moving average that gives more weight to recent prices.
    ///
    /// Reacts faster to price changes than a Simple Moving Average (SMA)
    /// while still filtering short-term noise.
    pub ema: Vec<f64>,

    /// Distance between the closing price and the EMA.
    ///
    /// Computed as:
    ///
    /// ```text
    /// Close - EMA
    /// ```
    ///
    /// Positive values indicate price is above the EMA.
    /// Negative values indicate price is below the EMA.
    ///
    /// The magnitude represents how far price has deviated
    /// from the current trend estimate.
    pub distance: Vec<f64>,

    /// First derivative of the EMA.
    ///
    /// Computed as:
    ///
    /// ```text
    /// EMA_t - EMA_(t-1)
    /// ```
    ///
    /// Positive values indicate an upward-sloping EMA.
    /// Negative values indicate a downward-sloping EMA.
    ///
    /// The magnitude represents trend acceleration and momentum.
    pub slope: Vec<f64>,
}

impl<const PERIOD: usize> ExpMovAvg<PERIOD> {
    /// Create a new empty [ExpMovAvg] instance.
    pub fn new() -> Self {
        Self {
            ema: Vec::new(),
            distance: Vec::new(),
            slope: Vec::new(),
        }
    }
}

impl<const PERIOD: usize> Indicator for ExpMovAvg<PERIOD> {
    fn name() -> String {
        format!("ema-{}", PERIOD)
    }

    fn compute(&mut self, ctx: Context) {
        let closes = &ctx.data().closes;
        let len = closes.len();

        self.ema = Vec::with_capacity(len);
        self.distance = Vec::with_capacity(len);
        self.slope = Vec::with_capacity(len);

        let alpha = 2.0 / (PERIOD as f64 + 1.0);

        let seed_len = PERIOD.min(len);
        let mut sum = 0.0;

        for (i, &close) in closes.iter().enumerate().take(seed_len) {
            sum += close;
            let sma = sum / (i + 1) as f64;

            self.ema.push(sma);
            self.distance.push(close - sma);
            self.slope
                .push(if i == 0 { 0.0 } else { sma - self.ema[i - 1] });
        }

        let mut ema = if seed_len > 0 {
            sum / seed_len as f64
        } else {
            0.0
        };

        let mut prev_ema = ema;

        for &close in closes.iter().skip(seed_len) {
            ema = alpha * close + (1.0 - alpha) * ema;

            self.ema.push(ema);
            self.distance.push(close - ema);
            self.slope.push(ema - prev_ema);

            prev_ema = ema;
        }
    }

    fn is_computed(&self) -> bool {
        !self.ema.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
