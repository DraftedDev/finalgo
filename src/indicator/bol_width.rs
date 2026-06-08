use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::math::{mean, rolling_min_max, std_dev};
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

const STD_MULTI: f64 = 2.0;

pub struct BollingerWidth<const PERIOD: usize> {
    pub width: Vec<f64>,
    pub min_max: Vec<f64>,
}

impl<const PERIOD: usize> BollingerWidth<PERIOD> {
    pub fn new() -> Self {
        Self {
            width: Vec::new(),
            min_max: Vec::new(),
        }
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

    fn score(&self, _: &Interface) -> Vec<ScoreRecord> {
        let mut out = Vec::new();

        let Some(&value) = self.min_max.last() else {
            return out;
        };

        if !value.is_finite() {
            return out;
        }

        let volatility_signal = (value * 2.0) - 1.0;

        out.push(ScoreRecord::new(
            ScoreType::Volatility,
            volatility_signal.clamp(-1.0, 1.0),
            1.0,
            0.8,
        ));

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
