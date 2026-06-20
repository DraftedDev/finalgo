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

        assert!(PERIOD > 0, "Stochastic PERIOD must be > 0");
        assert!(SMOOTH > 0, "Stochastic SMOOTH must be > 0");
        assert!(
            len >= PERIOD + SMOOTH - 1,
            "Must have at least {} + {} - 1 samples",
            PERIOD,
            SMOOTH
        );

        self.k = vec![f64::NAN; len];
        self.d = vec![f64::NAN; len];

        for (i, &close) in closes.iter().enumerate().skip(PERIOD - 1) {
            let start = i + 1 - PERIOD;

            let mut window_high = f64::NEG_INFINITY;
            let mut window_low = f64::INFINITY;

            for j in start..=i {
                let h = highs[j];
                let l = lows[j];

                if h.is_finite() && h > window_high {
                    window_high = h;
                }

                if l.is_finite() && l < window_low {
                    window_low = l;
                }
            }

            if !close.is_finite() {
                continue;
            }

            let range = (window_high - window_low).max(1e-12);
            self.k[i] = ((close - window_low) / range).clamp(0.0, 1.0);
        }

        for (i, d_val) in self.d.iter_mut().enumerate().skip(PERIOD + SMOOTH - 2) {
            let start = i + 1 - SMOOTH;
            let mut sum = 0.0;
            let mut valid = true;

            for j in start..=i {
                let val = self.k[j];
                if !val.is_finite() {
                    valid = false;
                    break;
                }
                sum += val;
            }

            if valid {
                *d_val = sum / SMOOTH as f64;
            }
        }
    }

    fn is_computed(&self) -> bool {
        !self.k.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
