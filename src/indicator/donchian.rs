use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::math::z_score;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

/// # Donchian Channel Position Indicator
///
/// ## Purpose
/// - Price location in recent range
/// - Breakout / breakdown detection
/// - Mean reversion context
///
/// ## Math
///
/// Donchian:
/// ```
/// upper = max(high[t-N..t])
/// lower = min(low[t-N..t])
///
/// POS_t = (close_t - lower) / (upper - lower)
/// POS_z = z_score(POS)
/// ```
pub struct DonchianPosition<const PERIOD: usize> {
    pub position: Vec<f64>,
    pub position_z: Vec<f64>,
    pub breakout: Vec<f64>,
}

impl<const PERIOD: usize> DonchianPosition<PERIOD> {
    pub fn new() -> Self {
        Self {
            position: Vec::new(),
            position_z: Vec::new(),
            breakout: Vec::new(),
        }
    }

    #[inline]
    fn safe_clamp01(x: f64) -> f64 {
        if !x.is_finite() {
            0.5
        } else {
            x.clamp(0.0, 1.0)
        }
    }
}

impl<const PERIOD: usize> Indicator for DonchianPosition<PERIOD> {
    fn name(&self) -> String {
        format!("donchian_pos-{}", PERIOD)
    }

    fn compute(&mut self, int: &Interface) {
        let data = int.raw();
        let highs = &data.highs;
        let lows = &data.lows;
        let closes = &data.closes;

        let len = closes.len();

        self.position = vec![0.5; len];
        self.breakout = vec![0.0; len];
        self.position_z = vec![0.0; len];

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

            let pos = if range > 1e-12 && window_high.is_finite() && window_low.is_finite() {
                (close - window_low) / range
            } else {
                0.5
            };

            self.position[i] = Self::safe_clamp01(pos);

            self.breakout[i] = if range > 1e-12 {
                ((close - window_high).max(window_low - close)) / range
            } else {
                0.0
            };
        }

        let clean_pos: Vec<f64> = self
            .position
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .collect();

        if clean_pos.len() > 5 {
            let z = z_score(&clean_pos);

            for i in 0..len {
                let v = self.position[i];

                self.position_z[i] = if v.is_finite() && !z.is_empty() {
                    z.get(i).copied().unwrap_or(0.0)
                } else {
                    0.0
                };
            }
        } else {
            self.position_z.fill(0.0);
        }
    }

    fn is_computed(&self) -> bool {
        !self.position.is_empty()
    }

    fn score(&self, _: &Interface) -> Vec<ScoreRecord> {
        let mut out = Vec::new();

        let idx = match self.position.iter().rposition(|v| v.is_finite()) {
            Some(i) => i,
            None => return out,
        };

        let pos = self.position[idx].clamp(0.0, 1.0);
        let pos_z = self.position_z[idx];

        let direction = ((pos - 0.5) * 2.0).clamp(-1.0, 1.0);

        let strength = ((pos - 0.5).abs() * 2.0).clamp(0.0, 1.0);

        let z_penalty = if pos_z.is_finite() {
            (pos_z.abs() / 3.0).min(1.0)
        } else {
            0.5
        };

        let quality = (1.0 - z_penalty).clamp(-1.0, 1.0);

        let confidence = strength * (1.0 - z_penalty).clamp(0.0, 1.0);

        let weight = 0.5 + strength * 0.4;

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

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
