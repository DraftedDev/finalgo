use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::math::{mean, z_score};
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

/// # Relative Volume Indicator
///
/// ## Purpose
/// - Participation Strength
/// - Breakout Confirmation
///
/// ## Math
/// ```text
/// RVOL_t = volume_t / mean(volume_{t-PERIOD...t})
/// RVOL_smooth = EMA(RVOL, α)
/// RVOL_z = z_score(RVOL)
/// ```
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

        // Raw RVOL
        for i in PERIOD..len {
            let window = &volumes[i - PERIOD..i];
            let avg = mean(window);

            self.vol[i] = if avg != 0.0 {
                volumes[i] / avg
            } else {
                1.0 // neutral instead of 0.0 (important fix)
            };
        }

        // Smoothed RVOL (EMA)
        let alpha = 2.0 / (PERIOD as f64 + 1.0);
        let mut ema = self.vol[PERIOD];

        for i in PERIOD..len {
            ema = alpha * self.vol[i] + (1.0 - alpha) * ema;
            self.vol_smoothed[i] = ema;
        }

        // Z-Score RVOL
        let clean_slice = &self.vol[PERIOD..];
        let z = z_score(clean_slice);

        for i in PERIOD..len {
            self.vol_z[i] = z[i - PERIOD];
        }
    }

    fn is_computed(&self) -> bool {
        !self.vol.is_empty()
    }

    fn score(&self) -> Vec<(ScoreType, ScoreRecord)> {
        fn last_finite(values: &[f64]) -> Option<f64> {
            values.iter().rev().copied().find(|v| v.is_finite())
        }

        let Some(smoothed) = last_finite(&self.vol_smoothed) else {
            return Vec::new();
        };

        let Some(z) = last_finite(&self.vol_z) else {
            return Vec::new();
        };

        let smoothed_component = ((smoothed - 1.0) / 1.5).clamp(-1.0, 1.0);

        let z_component = (z / 3.0).clamp(-1.0, 1.0);

        let quality = (0.7 * smoothed_component + 0.3 * z_component).clamp(-1.0, 1.0);

        vec![(ScoreType::Quality, ScoreRecord::new(quality, 1.0))]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
