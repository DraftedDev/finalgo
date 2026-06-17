use crate::engine::Context;
use crate::indicator::Indicator;
use std::any::Any;

/// # Stochastic Oscillator
///
/// Momentum oscillator that measures the position of the current close
/// relative to the recent high-low range.
///
/// Typically used to detect:
/// - Overbought / oversold conditions
/// - Momentum exhaustion
/// - Short-term reversal setups
pub struct Stochastic<const PERIOD: usize, const SMOOTH: usize> {
    /// %K line (raw stochastic value)
    ///
    /// Computed as:
    /// ```text
    /// %K = (Close - LowestLow(PERIOD)) / (HighestHigh(PERIOD) - LowestLow(PERIOD))
    /// ```
    ///
    /// Interprets where price sits inside the recent range:
    /// - 0.0 → at period low
    /// - 1.0 → at period high
    /// - values are clamped to [0, 1]
    pub k: Vec<f64>,

    /// %D line (smoothed stochastic)
    ///
    /// Simple moving average of %K over `SMOOTH` periods.
    ///
    /// Acts as a signal line:
    /// - smooths noise in %K
    /// - reacts slower to momentum changes
    /// - used for crossover-based signals in many strategies
    pub d: Vec<f64>,
}

impl<const PERIOD: usize, const SMOOTH: usize> Stochastic<PERIOD, SMOOTH> {
    /// Create a new empty [Stochastic] instance.
    pub fn new() -> Self {
        Self {
            k: Vec::new(),
            d: Vec::new(),
        }
    }

    #[inline]
    fn safe_range(high: f64, low: f64) -> f64 {
        (high - low).max(1e-12)
    }
}

impl<const PERIOD: usize, const SMOOTH: usize> Indicator for Stochastic<PERIOD, SMOOTH> {
    fn name() -> String {
        format!("stoch-{}-{}", PERIOD, SMOOTH)
    }

    fn compute(&mut self, ctx: Context) {
        let data = ctx.data();
        let closes = &data.closes;
        let highs = &data.highs;
        let lows = &data.lows;

        let len = closes.len();

        assert!(
            len >= PERIOD + SMOOTH,
            "Must have at least {PERIOD} + {SMOOTH} samples"
        );

        self.k = vec![f64::NAN; len];
        self.d = vec![f64::NAN; len];

        // %K calculation
        for i in PERIOD..len {
            let window_high = highs[i - PERIOD..i]
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);

            let window_low = lows[i - PERIOD..i]
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);

            let range = Self::safe_range(window_high, window_low);
            let close = closes[i];

            if !close.is_finite() {
                continue;
            }

            self.k[i] = ((close - window_low) / range).clamp(0.0, 1.0);
        }

        // %D smoothing (SMA), using current %K value
        for i in (PERIOD + SMOOTH - 1)..len {
            let slice = &self.k[i + 1 - SMOOTH..i + 1];

            if slice.iter().any(|v| !v.is_finite()) {
                continue;
            }

            self.d[i] = slice.iter().sum::<f64>() / SMOOTH as f64;
        }
    }

    fn is_computed(&self) -> bool {
        !self.k.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
