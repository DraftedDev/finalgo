use crate::engine::Context;
use crate::indicator::Indicator;
use crate::math::z_score;
use std::any::Any;

/// # Rate of Change Indicator
///
/// Measures the percentage price change over a fixed lookback period.
pub struct RateOfChange<const PERIOD: usize> {
    /// Rate of change over `PERIOD` bars.
    ///
    /// Computed as:
    ///
    /// ```text
    /// ROC = (close_t - close_{t-PERIOD}) / close_{t-PERIOD}
    /// ```
    ///
    /// Positive values indicate upward momentum.
    /// Negative values indicate downward momentum.
    pub roc: Vec<f64>,

    /// Absolute rate of change.
    ///
    /// Computed as:
    ///
    /// ```text
    /// |ROC|
    /// ```
    ///
    /// Represents magnitude of movement regardless of direction.
    /// Higher values indicate stronger momentum or volatility.
    pub roc_abs: Vec<f64>,

    /// Z-score normalized ROC.
    ///
    /// Measures how extreme the current ROC is relative to its historical distribution.
    ///
    /// Computed as:
    ///
    /// ```text
    /// z_score(roc)
    /// ```
    ///
    /// - Values near 0: normal momentum
    /// - Positive values: unusually strong upward momentum
    /// - Negative values: unusually strong downward momentum
    pub roc_z: Vec<f64>,
}

impl<const PERIOD: usize> RateOfChange<PERIOD> {
    /// Create a new empty [RateOfChange] instance.
    pub fn new() -> Self {
        Self {
            roc: Vec::new(),
            roc_abs: Vec::new(),
            roc_z: Vec::new(),
        }
    }
}

impl<const PERIOD: usize> Indicator for RateOfChange<PERIOD> {
    fn name() -> String {
        format!("roc-{}", PERIOD)
    }

    fn compute(&mut self, ctx: Context) {
        let closes = &ctx.data().closes;
        let len = closes.len();

        assert!(len >= PERIOD, "Must have at least {PERIOD} samples");

        self.roc = vec![0.0; len];
        self.roc_abs = vec![0.0; len];
        self.roc_z = vec![0.0; len];

        for i in PERIOD..len {
            let prev = closes[i - PERIOD];
            let curr = closes[i];

            let value = if prev.abs() > 1e-12 && prev.is_finite() && curr.is_finite() {
                (curr - prev) / prev
            } else {
                0.0
            };

            self.roc[i] = value;
            self.roc_abs[i] = value.abs();
        }

        self.roc_z = z_score(&self.roc);
    }

    fn is_computed(&self) -> bool {
        !self.roc.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
