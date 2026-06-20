use crate::engine::Context;
use crate::indicator::Indicator;
use std::any::Any;

/// # Relative Strength Index (RSI)
///
/// Momentum oscillator measuring the speed and magnitude of recent price changes.
/// Values are normalized to a symmetric range for downstream model usage.
pub struct RelStrengthIdx<const PERIOD: usize> {
    /// Normalized RSI values.
    ///
    /// Computed from classical RSI (0–100) and transformed into:
    ///
    /// ```text
    /// (RSI - 50) / 50
    /// ```
    ///
    /// Final range:
    /// - +1.0 → extremely overbought / strong upward momentum
    /// -  0.0 → neutral momentum
    /// - -1.0 → extremely oversold / strong downward momentum
    ///
    /// This normalization makes RSI compatible with other symmetric indicators
    /// like direction, stochastic, and BOS signals.
    pub rsi: Vec<f64>,
}

impl<const PERIOD: usize> RelStrengthIdx<PERIOD> {
    /// Create a new empty [RelStrengthIdx] instance.
    pub fn new() -> Self {
        Self { rsi: Vec::new() }
    }

    #[inline]
    fn compute_rsi(avg_gain: f64, avg_loss: f64) -> f64 {
        if avg_loss <= 1e-12 {
            return 100.0;
        }

        if avg_gain <= 1e-12 {
            return 0.0;
        }

        let rs = avg_gain / avg_loss;
        100.0 - (100.0 / (1.0 + rs))
    }

    #[inline]
    fn normalize(v: f64) -> f64 {
        ((v - 50.0) / 50.0).clamp(-1.0, 1.0)
    }
}

impl<const PERIOD: usize> Indicator for RelStrengthIdx<PERIOD> {
    fn name() -> String {
        format!("rsi-{}", PERIOD)
    }

    fn compute(&mut self, ctx: Context) {
        let closes = &ctx.data().closes;
        let len = closes.len();

        if len <= PERIOD {
            return;
        }

        self.rsi.resize(len, f64::NAN);

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in 1..=PERIOD {
            let diff = closes[i] - closes[i - 1];
            if diff >= 0.0 {
                gains += diff;
            } else {
                losses -= diff;
            }
        }

        let mut avg_gain = gains / PERIOD as f64;
        let mut avg_loss = losses / PERIOD as f64;

        let mut rsi = Self::compute_rsi(avg_gain, avg_loss);
        self.rsi[PERIOD] = Self::normalize(rsi);

        let period_f = PERIOD as f64;
        let period_minus_1 = period_f - 1.0;

        for i in (PERIOD + 1)..len {
            let diff = closes[i] - closes[i - 1];
            let gain = diff.max(0.0);
            let loss = (-diff).max(0.0);

            avg_gain = (avg_gain * period_minus_1 + gain) / period_f;
            avg_loss = (avg_loss * period_minus_1 + loss) / period_f;

            rsi = Self::compute_rsi(avg_gain, avg_loss);
            self.rsi[i] = Self::normalize(rsi);
        }
    }

    fn is_computed(&self) -> bool {
        !self.rsi.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
