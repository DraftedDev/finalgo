use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::math::{mean, rolling_min_max, std_dev};
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

const STD_MULTI: f64 = 2.0;

/// # Bollinger Band Width Indicator
///
/// ## Purpose
/// - Detect relative volatility compression/expansion.
/// - Identify squeeze and breakout environments.
/// - Measure volatility regime changes.
///
/// ## Math
///
/// ```
/// mean = SMA(window)
/// std = standard_deviation(window)
///
/// upper = mean + k * std
/// lower = mean - k * std
///
/// BOLL_W = (upper - lower) / mean
/// BOLL_W_MIN_MAX = rolling min-max normalization
/// ```
pub struct BollingerWidth<const PERIOD: usize> {
    width: Vec<f64>,
    min_max: Vec<f64>,
}

impl<const PERIOD: usize> BollingerWidth<PERIOD> {
    pub fn new() -> Self {
        Self {
            width: Vec::new(),
            min_max: Vec::new(),
        }
    }

    pub fn min_max(&self) -> &[f64] {
        &self.min_max
    }
}

impl<const PERIOD: usize> Indicator for BollingerWidth<PERIOD> {
    fn name(&self) -> String {
        format!("bollinger-width-{}", PERIOD)
    }

    fn compute(&mut self, int: &Interface) {
        let closes = &int.raw().closes;

        self.width = vec![f64::NAN; closes.len()];

        for i in (PERIOD - 1)..closes.len() {
            let window = &closes[i + 1 - PERIOD..=i];

            let mean = mean(window);
            let std = std_dev(window, mean);

            let upper = mean + STD_MULTI * std;
            let lower = mean - STD_MULTI * std;

            self.width[i] = if mean != 0.0 {
                (upper - lower) / mean
            } else {
                0.0
            };
        }

        self.min_max = rolling_min_max(&self.width, 100);
    }

    fn is_computed(&self) -> bool {
        !self.width.is_empty()
    }

    fn score(&self) -> Vec<(ScoreType, ScoreRecord)> {
        let mut out = Vec::new();

        if self.min_max().is_empty() {
            return out;
        }

        let v = *self.min_max().last().unwrap();

        if !v.is_finite() {
            return out;
        }
        let regime = (v * 2.0 - 1.0).clamp(-1.0, 1.0);

        let confidence = ((v - 0.5).abs() * 2.0).clamp(0.0, 1.0);

        out.push((ScoreType::Volatility, ScoreRecord::new(regime, confidence)));

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
