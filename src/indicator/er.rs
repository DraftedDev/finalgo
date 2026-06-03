use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

/// # Efficiency Ratio Indicator
///
/// ## Purpose
/// - Trend Quality vs Noise
///
/// ## Math
/// ```
/// ER_t = |close_t - close_{t-n}| / Σ |close_i - close_{i-1}|
/// ER_smooth_t = α · ER_t + (1 - α) · ER_smooth_{t-1}
/// ER_slope_t = ER_smooth_t - ER_smooth_{t-1}
/// ER_accel_t = ER_slope_t - ER_slope_{t-1}
/// ```
pub struct EfficiencyRatio<const PERIOD: usize> {
    pub er: Vec<f64>,
    pub smoothed: Vec<f64>,
    pub slope: Vec<f64>,
    pub accel: Vec<f64>,
}

impl<const PERIOD: usize> EfficiencyRatio<PERIOD> {
    pub fn new() -> Self {
        Self {
            er: Vec::new(),
            smoothed: Vec::new(),
            slope: Vec::new(),
            accel: Vec::new(),
        }
    }

    fn latest_finite(values: &[f64]) -> f64 {
        values
            .iter()
            .rev()
            .copied()
            .find(|v| v.is_finite())
            .unwrap_or(0.0)
    }
}
impl<const PERIOD: usize> Indicator for EfficiencyRatio<PERIOD> {
    fn name(&self) -> String {
        format!("er-{}", PERIOD)
    }

    fn compute(&mut self, int: &Interface) {
        let closes = &int.raw().closes;
        let len = closes.len();

        self.er = vec![f64::NAN; len];
        self.smoothed = vec![f64::NAN; len];
        self.slope = vec![f64::NAN; len];
        self.accel = vec![f64::NAN; len];

        for i in PERIOD..len {
            let numerator = (closes[i] - closes[i - PERIOD]).abs();

            let mut denominator = 0.0;
            for j in (i - PERIOD + 1)..=i {
                denominator += (closes[j] - closes[j - 1]).abs();
            }

            self.er[i] = if denominator != 0.0 {
                numerator / denominator
            } else {
                0.0
            };
        }

        let smooth_period = 3;
        let alpha = 2.0 / (smooth_period as f64 + 1.0);

        let mut smooth = Self::latest_finite(&self.er);

        for i in 0..len {
            let er_i = if self.er[i].is_finite() {
                self.er[i]
            } else {
                smooth
            };
            smooth = alpha * er_i + (1.0 - alpha) * smooth;
            self.smoothed[i] = smooth;
        }

        for i in 1..len {
            if self.smoothed[i].is_finite() && self.smoothed[i - 1].is_finite() {
                self.slope[i] = self.smoothed[i] - self.smoothed[i - 1];
            }
        }

        for i in 2..len {
            if self.slope[i].is_finite() && self.slope[i - 1].is_finite() {
                self.accel[i] = self.slope[i] - self.slope[i - 1];
            }
        }
    }

    fn is_computed(&self) -> bool {
        !self.er.is_empty()
    }

    fn score(&self) -> Vec<ScoreRecord> {
        let mut out = Vec::with_capacity(3);

        let er = Self::latest_finite(&self.smoothed);
        let slope = Self::latest_finite(&self.slope);
        let accel = Self::latest_finite(&self.accel);

        let quality = (er * 2.0) - 1.0;

        out.push(ScoreRecord::new(
            ScoreType::Quality,
            quality.clamp(-1.0, 1.0),
            0.8,
            er.clamp(0.0, 1.0),
        ));

        let direction = (slope * 10.0).tanh();

        out.push(ScoreRecord::new(
            ScoreType::Direction,
            direction.clamp(-1.0, 1.0),
            0.6,
            er.clamp(0.0, 1.0),
        ));

        let volatility = (accel * 10.0).tanh();

        out.push(ScoreRecord::new(
            ScoreType::Volatility,
            volatility.clamp(-1.0, 1.0),
            0.5,
            er.clamp(0.0, 1.0),
        ));

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
