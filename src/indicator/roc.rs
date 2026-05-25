use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::math::z_score;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

/// # Rate of Change Indicator
///
/// ## Purpose
/// - Momentum
/// - Price Acceleration
///
/// ## Math
/// ```
/// ROC_t = (close_t - close_{t-n}) / close_{t-n}
/// ROC_ABS = |ROC|
/// ROC_Z = z_score(ROC)
/// ```
pub struct RateOfChange<const PERIOD: usize> {
    pub roc: Vec<f64>,
    pub roc_abs: Vec<f64>,
    pub roc_z: Vec<f64>,
}

impl<const PERIOD: usize> RateOfChange<PERIOD> {
    pub fn new() -> Self {
        Self {
            roc: Vec::new(),
            roc_abs: Vec::new(),
            roc_z: Vec::new(),
        }
    }
}

impl<const PERIOD: usize> Indicator for RateOfChange<PERIOD> {
    fn name(&self) -> String {
        format!("roc-{}", PERIOD)
    }

    fn compute(&mut self, int: &Interface) {
        let closes = &int.raw().closes;
        let len = closes.len();

        self.roc = vec![f64::NAN; len];
        self.roc_abs = vec![f64::NAN; len];

        for i in PERIOD..len {
            let prev = closes[i - PERIOD];
            let curr = closes[i];

            let value = if prev != 0.0 {
                (curr - prev) / prev
            } else {
                f64::NAN
            };

            self.roc[i] = value;
            self.roc_abs[i] = value.abs();
        }

        // IMPORTANT: z-score should ignore NaN
        self.roc_z = z_score(&self.roc);
    }

    fn is_computed(&self) -> bool {
        !self.roc.is_empty()
    }

    fn score(&self) -> Vec<(ScoreType, ScoreRecord)> {
        let mut out = Vec::new();

        if self.roc.is_empty() || self.roc_z.is_empty() {
            return out;
        }

        let len = self.roc.len();

        for i in 0..len {
            let roc = self.roc[i];
            let roc_z = self.roc_z[i];

            if !roc.is_finite() || !roc_z.is_finite() {
                continue;
            }

            let direction = roc.tanh();

            let strength = (roc_z.abs() / 3.0).tanh();
            let weight = roc_z.abs().tanh().clamp(0.05, 1.0);

            out.push((
                ScoreType::Direction,
                ScoreRecord::new(direction.clamp(-1.0, 1.0), weight),
            ));

            out.push((
                ScoreType::Strength,
                ScoreRecord::new(strength.clamp(0.0, 1.0), weight),
            ));
        }

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
