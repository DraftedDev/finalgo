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
}

impl<const PERIOD: usize> DonchianPosition<PERIOD> {
    pub fn new() -> Self {
        Self {
            position: Vec::new(),
            position_z: Vec::new(),
        }
    }

    pub fn position(&self) -> &[f64] {
        &self.position
    }

    pub fn position_z(&self) -> &[f64] {
        &self.position_z
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

        self.position = vec![f64::NAN; len];

        for i in PERIOD..len {
            let window_high = highs[i - PERIOD..i]
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);

            let window_low = lows[i - PERIOD..i]
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);

            let range = window_high - window_low;

            let pos = if range != 0.0 {
                (closes[i] - window_low) / range
            } else {
                0.5
            };

            self.position[i] = pos.clamp(0.0, 1.0);
        }

        self.position_z = z_score(&self.position);
    }

    fn is_computed(&self) -> bool {
        !self.position.is_empty()
    }

    fn score(&self) -> Vec<(ScoreType, ScoreRecord)> {
        let mut out = Vec::new();

        let pos = self.position().last().copied().unwrap_or(f64::NAN);
        let pos_z = self.position_z().last().copied().unwrap_or(f64::NAN);

        if !pos.is_finite() || !pos_z.is_finite() {
            return out;
        }

        let direction = ((pos - 0.5) * 2.0).clamp(-1.0, 1.0);
        out.push((ScoreType::Direction, ScoreRecord::new(direction, 0.70)));

        let strength = ((pos - 0.5).abs() * 2.0).clamp(0.0, 1.0);
        out.push((ScoreType::Strength, ScoreRecord::new(strength, 0.55)));

        let quality = (pos_z.abs() / 2.5).clamp(0.0, 1.0);
        out.push((ScoreType::Quality, ScoreRecord::new(quality, 0.35)));

        let volatility = (pos_z / 3.0).clamp(-1.0, 1.0);
        out.push((ScoreType::Volatility, ScoreRecord::new(volatility, 0.20)));

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
