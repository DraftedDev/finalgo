use crate::engine::Context;
use crate::indicator::Indicator;
use std::any::Any;

/// # Average True Range (Wilder)
///
/// Measures market volatility using True Range and Wilder smoothing.
pub struct AvgTrueRange<const PERIOD: usize> {
    /// True Range (TR) for each candle.
    ///
    /// Represents the largest of:
    /// - current high - current low
    /// - abs(current high - previous close)
    /// - abs(current low - previous close)
    ///
    /// Higher values indicate larger price movement during the candle.
    pub tr: Vec<f64>,

    /// Average True Range (ATR).
    ///
    /// Wilder-smoothed moving average of `tr`.
    ///
    /// Represents the average absolute price movement over time.
    /// Larger values indicate a more volatile market.
    pub atr: Vec<f64>,

    /// Normalized ATR.
    ///
    /// Computed as:
    ///
    /// ```text
    /// ATR / Close
    /// ```
    ///
    /// Expresses volatility relative to the current price,
    /// making values comparable across assets with different prices.
    pub norm_atr: Vec<f64>,
}

impl<const PERIOD: usize> AvgTrueRange<PERIOD> {
    /// Creates a new [AvgTrueRange] instance.
    pub fn new() -> Self {
        Self {
            tr: Vec::new(),
            atr: Vec::new(),
            norm_atr: Vec::new(),
        }
    }
}

impl<const PERIOD: usize> Indicator for AvgTrueRange<PERIOD> {
    fn name() -> String {
        format!("atr-{}", PERIOD)
    }

    fn compute(&mut self, ctx: Context) {
        let data = ctx.data();

        let highs = &data.highs;
        let lows = &data.lows;
        let closes = &data.closes;

        let len = closes.len();

        assert!(len >= PERIOD, "Need at least {PERIOD} samples");

        self.tr = vec![0.0; len];
        self.atr = vec![f64::NAN; len];
        self.norm_atr = vec![f64::NAN; len];

        // True Range
        self.tr[0] = highs[0] - lows[0];

        for i in 1..len {
            let high = highs[i];
            let low = lows[i];
            let prev_close = closes[i - 1];

            self.tr[i] = (high - low)
                .max((high - prev_close).abs())
                .max((low - prev_close).abs());
        }

        // Seed ATR using SMA(TR)
        let seed = self.tr[..PERIOD].iter().copied().sum::<f64>() / PERIOD as f64;

        self.atr[PERIOD - 1] = seed;

        // Wilder smoothing
        let mut atr = seed;

        for i in PERIOD..len {
            atr = ((atr * (PERIOD as f64 - 1.0)) + self.tr[i]) / PERIOD as f64;

            self.atr[i] = atr;
        }

        // Normalized ATR
        for (i, &close) in closes.iter().enumerate().skip(PERIOD - 1) {
            if close.abs() > 1e-12 {
                self.norm_atr[i] = self.atr[i] / close;
            }
        }
    }

    fn is_computed(&self) -> bool {
        !self.atr.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
