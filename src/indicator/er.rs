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

    fn mean_last_n(values: &[f64], n: usize) -> f64 {
        let start = values.len().saturating_sub(n);
        let slice = &values[start..];
        let mut sum = 0.0;
        let mut count = 0;

        for v in slice {
            if v.is_finite() {
                sum += *v;
                count += 1;
            }
        }

        if count == 0 { 0.0 } else { sum / count as f64 }
    }

    fn stability(values: &[f64], n: usize) -> f64 {
        let start = values.len().saturating_sub(n);
        let slice = &values[start..];

        let mut mean = 0.0;
        let mut count = 0;

        for v in slice {
            if v.is_finite() {
                mean += *v;
                count += 1;
            }
        }

        if count == 0 {
            return 0.0;
        }

        mean /= count as f64;

        let mut var = 0.0;
        for v in slice {
            if v.is_finite() {
                let d = v - mean;
                var += d * d;
            }
        }

        1.0 / (1.0 + (var / count as f64).sqrt())
    }
}

impl<const PERIOD: usize> Indicator for EfficiencyRatio<PERIOD> {
    fn name(&self) -> String {
        format!("er-{}", PERIOD)
    }

    fn compute(&mut self, int: &Interface) {
        let closes = &int.raw().closes;
        let len = closes.len();

        self.er = vec![0.0; len];
        self.smoothed = vec![0.0; len];
        self.slope = vec![0.0; len];
        self.accel = vec![0.0; len];

        for i in PERIOD..len {
            let numerator = (closes[i] - closes[i - PERIOD]).abs();

            let mut denom = 0.0;
            for j in (i - PERIOD + 1)..=i {
                denom += (closes[j] - closes[j - 1]).abs();
            }

            self.er[i] = if denom > 0.0 { numerator / denom } else { 0.0 };
        }

        let alpha = 0.5;
        let mut smooth = self.er[0];

        for i in 0..len {
            smooth = alpha * self.er[i] + (1.0 - alpha) * smooth;
            self.smoothed[i] = smooth;
        }

        for i in 1..len {
            self.slope[i] = self.smoothed[i] - self.smoothed[i - 1];
        }

        for i in 2..len {
            self.accel[i] = self.slope[i] - self.slope[i - 1];
        }
    }

    fn is_computed(&self) -> bool {
        !self.er.is_empty()
    }

    fn score(&self, _: &Interface) -> Vec<ScoreRecord> {
        let window = 10;

        let er_mean = Self::mean_last_n(&self.smoothed, window);
        let slope = Self::mean_last_n(&self.slope, window);
        let accel = Self::mean_last_n(&self.accel, window);

        let quality = (er_mean * 2.0 - 1.0).tanh();

        let direction = (slope * 8.0 + accel * 4.0).tanh();

        let strength = (er_mean * 0.6 + slope.abs() * 0.4).clamp(0.0, 1.0);

        let confidence = Self::stability(&self.smoothed, window);

        vec![
            ScoreRecord::new(
                ScoreType::Quality,
                quality.clamp(-1.0, 1.0),
                0.7,
                confidence,
            ),
            ScoreRecord::new(
                ScoreType::Direction,
                direction.clamp(-1.0, 1.0),
                0.6,
                confidence,
            ),
            ScoreRecord::new(ScoreType::Strength, strength, 0.5, confidence),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
