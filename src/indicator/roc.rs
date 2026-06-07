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

            if prev.abs() < 1e-12 || !prev.is_finite() || !curr.is_finite() {
                continue;
            }

            let value = (curr - prev) / prev;

            self.roc[i] = value;
            self.roc_abs[i] = value.abs();
        }

        // robust normalization (ignores NaN)
        self.roc_z = z_score(&self.roc);
    }

    fn is_computed(&self) -> bool {
        !self.roc.is_empty()
    }

    fn score(&self, _: &Interface) -> Vec<ScoreRecord> {
        let len = self.roc.len();
        if len == 0 {
            return vec![];
        }

        let i = len - 1;

        let roc = self.roc[i];
        let roc_abs = self.roc_abs[i];
        let roc_z = self.roc_z[i];

        if !roc.is_finite() || !roc_abs.is_finite() || !roc_z.is_finite() {
            return vec![];
        }

        let direction = (roc * 8.0).tanh();

        let mut out = Vec::with_capacity(3);

        out.push(ScoreRecord::new(
            ScoreType::Direction,
            direction,
            0.95, // ROC is core momentum signal
            1.0,
        ));

        let strength = (roc_abs * 6.0).tanh();

        out.push(ScoreRecord::new(
            ScoreType::Strength,
            strength.clamp(0.0, 1.0),
            0.85,
            1.0,
        ));

        let quality = (-(roc_z.abs() * 0.7)).tanh();

        out.push(ScoreRecord::new(
            ScoreType::Quality,
            quality.clamp(-1.0, 1.0),
            0.7,
            1.0,
        ));

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
