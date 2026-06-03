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

    fn score(&self) -> Vec<ScoreRecord> {
        let mut out = Vec::new();

        let len = self.position.len();

        for i in PERIOD..len {
            let pos = self.position[i];
            let pos_z = self.position_z[i];

            if pos.is_nan() || pos_z.is_nan() {
                continue;
            }

            let direction = (pos - 0.5) * 2.0;

            out.push(ScoreRecord::new(
                ScoreType::Direction,
                direction.clamp(-1.0, 1.0),
                0.8,
                0.7,
            ));

            let strength = (pos - 0.5).abs() * 2.0;

            out.push(ScoreRecord::new(
                ScoreType::Strength,
                strength.clamp(0.0, 1.0),
                0.9,
                0.7,
            ));

            let quality = (1.0 - (pos_z.abs() / 3.0)).clamp(-1.0, 1.0);

            out.push(ScoreRecord::new(ScoreType::Quality, quality, 0.6, 0.6));
        }

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
