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

    fn score(&self) -> Vec<ScoreRecord> {
        let len = self.vol.len();
        let mut out = Vec::with_capacity(len);

        for i in 0..len {
            let vol = self.vol[i];
            let smooth = self.vol_smoothed[i];
            let z = self.vol_z[i];

            if !vol.is_finite() || !smooth.is_finite() || !z.is_finite() {
                continue;
            }

            let strength = (vol - 1.0).tanh(); // [-1, 1]

            let strength = (strength + 1.0) / 2.0; // convert to [0,1]

            out.push(ScoreRecord::new(
                ScoreType::Strength,
                strength.clamp(0.0, 1.0),
                0.5, // medium-high importance
                0.8, // fairly reliable
            ));

            let stability = if vol != 0.0 {
                1.0 - ((smooth - vol).abs() / vol).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let quality = (stability * 2.0 - 1.0).clamp(-1.0, 1.0);

            out.push(ScoreRecord::new(ScoreType::Quality, quality, 0.3, 0.7));

            let direction = z.tanh(); // compress extreme spikes

            out.push(ScoreRecord::new(
                ScoreType::Direction,
                direction.clamp(-1.0, 1.0),
                0.2,
                0.6,
            ));
        }

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
