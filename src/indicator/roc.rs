use crate::engine::Context;
use crate::indicator::Indicator;
use std::any::Any;

/// Rolling window for Z-score normalization (~6 months of trading days).
const Z_WINDOW: usize = 120;

/// # Rate of Change Indicator
///
/// Measures the percentage price change over a fixed lookback period.
pub struct RateOfChange<const PERIOD: usize> {
    /// Rate of change over `PERIOD` bars.
    pub roc: Vec<f64>,

    /// Absolute rate of change.
    pub roc_abs: Vec<f64>,

    /// Rolling Z-score normalized ROC.
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

        assert!(len >= PERIOD, "Must have at least {PERIOD} samples for ROC");

        self.roc.reserve(len);
        self.roc_abs.reserve(len);
        self.roc_z.reserve(len);

        let mut ring = [0.0; Z_WINDOW];
        let mut ring_idx = 0;
        let mut sum = 0.0;
        let mut sq_sum = 0.0;
        let mut count = 0;

        for i in 0..len {
            if i < PERIOD {
                self.roc.push(0.0);
                self.roc_abs.push(0.0);
                self.roc_z.push(0.0);
                continue;
            }

            let prev = closes[i - PERIOD];
            let curr = closes[i];

            let value = if prev.abs() > 1e-12 && prev.is_finite() && curr.is_finite() {
                (curr - prev) / prev
            } else {
                0.0
            };

            self.roc.push(value);
            self.roc_abs.push(value.abs());

            let old = ring[ring_idx];
            sum -= old;
            sq_sum -= old * old;

            ring[ring_idx] = value;
            sum += value;
            sq_sum += value * value;

            if count < Z_WINDOW {
                count += 1;
            }
            ring_idx = (ring_idx + 1) % Z_WINDOW;

            let mean = sum / count as f64;
            let variance = ((sq_sum / count as f64) - (mean * mean)).max(0.0);
            let std_dev = variance.sqrt().max(1e-12);

            let z = (value - mean) / std_dev;
            self.roc_z.push(z);
        }
    }

    fn is_computed(&self) -> bool {
        !self.roc.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
