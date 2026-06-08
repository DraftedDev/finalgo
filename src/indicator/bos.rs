use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

pub struct BreakOfStruct<const LOOKBACK: usize> {
    direction: Vec<f64>,
    strength: Vec<f64>,
    quality: Vec<f64>,
}

impl<const LOOKBACK: usize> BreakOfStruct<LOOKBACK> {
    pub fn new() -> Self {
        Self {
            direction: Vec::new(),
            strength: Vec::new(),
            quality: Vec::new(),
        }
    }
}

impl<const LOOKBACK: usize> Indicator for BreakOfStruct<LOOKBACK> {
    fn name(&self) -> String {
        format!("bos-{}", LOOKBACK)
    }

    fn compute(&mut self, int: &Interface) {
        let data = int.raw();
        let highs = &data.highs;
        let lows = &data.lows;
        let closes = &data.closes;

        let len = closes.len();

        self.direction = vec![0.0; len];
        self.strength = vec![0.0; len];
        self.quality = vec![0.0; len];

        for i in LOOKBACK..len {
            let window_high = highs[i - LOOKBACK..i]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);

            let window_low = lows[i - LOOKBACK..i]
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);

            let close = closes[i];

            if !close.is_finite() {
                continue;
            }

            let mut direction = 0.0;
            let mut strength = 0.0;
            let mut quality = 0.0;

            // --- Bullish BOS ---
            if close > window_high {
                let extension = (close - window_high) / window_high.max(1e-12);

                direction = 1.0;
                strength = extension.clamp(0.0, 1.0);

                // quality = how clean break is (no wick back below level approximation)
                let rejection = (closes[i - 1] - window_high).abs() / window_high.max(1e-12);
                quality = (1.0 - rejection).clamp(-1.0, 1.0);
            }
            // --- Bearish BOS ---
            else if close < window_low {
                let extension = (window_low - close) / window_low.max(1e-12);

                direction = -1.0;
                strength = extension.clamp(0.0, 1.0);

                let rejection = (window_low - closes[i - 1]).abs() / window_low.max(1e-12);
                quality = (1.0 - rejection).clamp(-1.0, 1.0);
            }

            self.direction[i] = direction;
            self.strength[i] = strength;
            self.quality[i] = quality;
        }
    }

    fn is_computed(&self) -> bool {
        !self.direction.is_empty()
    }

    fn score(&self, _: &Interface) -> Vec<ScoreRecord> {
        let mut out = Vec::new();

        let idx = match self.direction.iter().rposition(|v| *v != 0.0) {
            Some(i) => i,
            None => return out,
        };

        let direction = self.direction[idx];
        let strength = self.strength[idx];
        let quality = self.quality[idx];

        if direction == 0.0 {
            return out;
        }

        let confidence = (strength * 0.6 + quality * 0.4).clamp(0.0, 1.0);

        let weight = 1.0;

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
