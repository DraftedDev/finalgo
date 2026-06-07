use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::math::mean;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

/// # Stochastic Oscillator Indicator
///
/// ## Purpose
/// - Position within range
/// - Overbought / oversold pressure
/// - Exhaustion detection
///
/// ## Math
/// ```
/// %K = (close - low_n) / (high_n - low_n)
/// %D = SMA(%K)
/// ```
pub struct Stochastic<const PERIOD: usize, const SMOOTH: usize> {
    pub k: Vec<f64>,
    pub d: Vec<f64>,
}

impl<const PERIOD: usize, const SMOOTH: usize> Stochastic<PERIOD, SMOOTH> {
    pub fn new() -> Self {
        Self {
            k: Vec::new(),
            d: Vec::new(),
        }
    }
}

impl<const PERIOD: usize, const SMOOTH: usize> Indicator for Stochastic<PERIOD, SMOOTH> {
    fn name(&self) -> String {
        format!("stoch-{}", PERIOD)
    }

    fn compute(&mut self, int: &Interface) {
        let data = int.raw();
        let closes = &data.closes;
        let highs = &data.highs;
        let lows = &data.lows;

        let len = closes.len();

        self.k = vec![f64::NAN; len];
        self.d = vec![f64::NAN; len];

        // %K
        for i in PERIOD..len {
            let window_high = highs[i - PERIOD..i]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);

            let window_low = lows[i - PERIOD..i]
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);

            let range = window_high - window_low;

            let close = closes[i];

            let k = if range > 1e-12 {
                ((close - window_low) / range).clamp(0.0, 1.0)
            } else {
                0.5
            };

            self.k[i] = k;
        }

        // %D smoothing
        for i in (PERIOD + SMOOTH)..len {
            let slice = &self.k[i - SMOOTH..i];

            if slice.iter().any(|v| !v.is_finite()) {
                self.d[i] = f64::NAN;
            } else {
                self.d[i] = mean(slice);
            }
        }
    }

    fn is_computed(&self) -> bool {
        !self.k.is_empty()
    }

    fn score(&self, _: &Interface) -> Vec<ScoreRecord> {
        let mut out = Vec::new();
        let len = self.k.len().min(self.d.len());

        for i in 0..len {
            let k = self.k[i];
            let d = self.d[i];

            if !k.is_finite() || !d.is_finite() {
                continue;
            }

            let direction = ((k - 0.5) * 2.2).clamp(-1.0, 1.0);

            let strength = ((k - 0.5).abs() * 2.0).clamp(0.0, 1.0);

            let divergence = (k - d).abs();

            let quality = (1.0 - divergence * 2.0).clamp(-1.0, 1.0);

            let confidence = (strength * (1.0 - divergence)).clamp(0.0, 1.0);

            let weight = 0.3 + (1.0 - strength) * 0.5;

            out.push(ScoreRecord::new(
                ScoreType::Direction,
                direction,
                weight,
                confidence,
            ));

            out.push(ScoreRecord::new(
                ScoreType::Strength,
                strength,
                weight,
                confidence,
            ));

            out.push(ScoreRecord::new(
                ScoreType::Quality,
                quality,
                weight,
                confidence,
            ));
        }

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
