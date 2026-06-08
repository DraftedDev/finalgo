use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::math::mean;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

pub struct RelativeVolume<const PERIOD: usize> {
    pub vol: Vec<f64>,
    pub vol_smoothed: Vec<f64>,
    pub vol_z: Vec<f64>,
}

impl<const PERIOD: usize> RelativeVolume<PERIOD> {
    pub fn new() -> Self {
        Self {
            vol: Vec::new(),
            vol_smoothed: Vec::new(),
            vol_z: Vec::new(),
        }
    }

    fn mean_last(values: &[f64], n: usize) -> f64 {
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

impl<const PERIOD: usize> Indicator for RelativeVolume<PERIOD> {
    fn name(&self) -> String {
        format!("relative-volume-{}", PERIOD)
    }

    fn compute(&mut self, int: &Interface) {
        let volumes = &int.raw().volumes;
        let len = volumes.len();

        self.vol = vec![0.0; len];
        self.vol_smoothed = vec![0.0; len];
        self.vol_z = vec![0.0; len];

        for i in PERIOD..len {
            let window = &volumes[i - PERIOD..i];
            let avg = mean(window);

            self.vol[i] = if avg > 0.0 { volumes[i] / avg } else { 1.0 };
        }

        let alpha = 2.0 / (PERIOD as f64 + 1.0);
        let mut ema = 1.0;

        for i in 0..len {
            ema = alpha * self.vol[i] + (1.0 - alpha) * ema;
            self.vol_smoothed[i] = ema;
        }

        let mut mean = 0.0;
        let mut count = 0;

        for i in PERIOD..len {
            mean += self.vol[i];
            count += 1;
        }

        mean /= count.max(1) as f64;

        let mut var = 0.0;
        for i in PERIOD..len {
            let d = self.vol[i] - mean;
            var += d * d;
        }

        let std = (var / count.max(1) as f64).sqrt().max(1e-8);

        for i in PERIOD..len {
            self.vol_z[i] = (self.vol[i] - mean) / std;
        }
    }

    fn is_computed(&self) -> bool {
        !self.vol.is_empty()
    }

    fn score(&self, _: &Interface) -> Vec<ScoreRecord> {
        let window = 10;

        let rvol = Self::mean_last(&self.vol_smoothed, window);
        let z = Self::mean_last(&self.vol_z, window);
        let stability = Self::stability(&self.vol_smoothed, window);

        let strength = ((rvol - 1.0) * 0.8).tanh();

        let quality = (stability * 2.0 - 1.0).tanh();

        let direction = (z * 0.5 + (rvol - 1.0) * 0.5).tanh();

        let confidence = stability;

        vec![
            ScoreRecord::new(
                ScoreType::Strength,
                ((strength + 1.0) / 2.0).clamp(0.0, 1.0),
                0.6,
                confidence,
            ),
            ScoreRecord::new(
                ScoreType::Quality,
                quality.clamp(-1.0, 1.0),
                0.4,
                confidence,
            ),
            ScoreRecord::new(
                ScoreType::Direction,
                direction.clamp(-1.0, 1.0),
                0.3,
                confidence,
            ),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
