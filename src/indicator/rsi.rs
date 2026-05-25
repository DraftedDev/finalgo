use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

/// # Relative Strength Index (RSI)
///
/// ## Purpose
/// - Momentum + mean reversion pressure
/// - Overbought / oversold detection (used structurally, not literally)
///
/// ## Math
/// ```
/// gain = max(Δclose, 0)
/// loss = max(-Δclose, 0)
///
/// avg_gain = WilderSMA(gain, period)
/// avg_loss = WilderSMA(loss, period)
///
/// RS = avg_gain / avg_loss
/// RSI = 100 - (100 / (1 + RS))
///
/// RSI_slope = RSI_t - RSI_{t-1}
/// ```
pub struct RelStrengthIdx<const PERIOD: usize> {
    pub rsi: Vec<f64>,
    pub rsi_slope: Vec<f64>,
}

impl<const PERIOD: usize> RelStrengthIdx<PERIOD> {
    pub fn new() -> Self {
        Self {
            rsi: Vec::new(),
            rsi_slope: Vec::new(),
        }
    }
}

impl<const PERIOD: usize> Indicator for RelStrengthIdx<PERIOD> {
    fn name(&self) -> String {
        format!("rsi-{}", PERIOD)
    }

    fn compute(&mut self, int: &Interface) {
        let closes = &int.raw().closes;
        let len = closes.len();

        self.rsi = vec![f64::NAN; len];
        self.rsi_slope = vec![f64::NAN; len];

        if len <= PERIOD + 1 {
            return;
        }

        let mut gains = vec![0.0; len];
        let mut losses = vec![0.0; len];

        for i in 1..len {
            let change = closes[i] - closes[i - 1];

            if change > 0.0 {
                gains[i] = change;
            } else {
                losses[i] = -change;
            }
        }

        // Seed average
        let mut avg_gain = 0.0;
        let mut avg_loss = 0.0;

        for i in 1..=PERIOD {
            avg_gain += gains[i];
            avg_loss += losses[i];
        }

        avg_gain /= PERIOD as f64;
        avg_loss /= PERIOD as f64;

        // First valid RSI index
        let mut prev_rsi = f64::NAN;

        for i in PERIOD..len {
            avg_gain = (avg_gain * (PERIOD as f64 - 1.0) + gains[i]) / PERIOD as f64;
            avg_loss = (avg_loss * (PERIOD as f64 - 1.0) + losses[i]) / PERIOD as f64;

            // RS Calculation
            let rs = match (avg_gain, avg_loss) {
                (0.0, 0.0) => 50.0,
                (_, 0.0) => f64::INFINITY,
                (0.0, _) => 0.0,
                _ => avg_gain / avg_loss,
            };

            let rsi = if rs.is_infinite() {
                100.0
            } else {
                100.0 - (100.0 / (1.0 + rs))
            };

            self.rsi[i] = rsi;

            // RSI Slope
            if !prev_rsi.is_nan() {
                self.rsi_slope[i] = rsi - prev_rsi;
            }

            prev_rsi = rsi;
        }
    }

    fn is_computed(&self) -> bool {
        !self.rsi.is_empty()
    }

    fn score(&self) -> Vec<(ScoreType, ScoreRecord)> {
        let len = self.rsi.len();

        if len == 0 {
            return vec![];
        }

        let mut out = Vec::with_capacity(len);

        for i in 0..len {
            let rsi = self.rsi[i];
            let slope = self.rsi_slope[i];

            if !rsi.is_finite() {
                continue;
            }

            let direction = ((rsi - 50.0) / 50.0).clamp(-1.0, 1.0);
            let strength = ((rsi - 50.0).abs() / 50.0).clamp(0.0, 1.0);

            let quality = if slope.is_finite() {
                let normalized_slope = slope.abs().clamp(0.0, 10.0) / 10.0;
                let clean = 1.0 - normalized_slope;

                clean.clamp(-1.0, 1.0)
            } else {
                0.0
            };

            let volatility = {
                let dist = (rsi - 50.0).abs() / 50.0;
                dist.clamp(-1.0, 1.0)
            };

            out.push((ScoreType::Direction, ScoreRecord::new(direction, 1.0)));
            out.push((ScoreType::Strength, ScoreRecord::new(strength, 1.0)));
            out.push((ScoreType::Quality, ScoreRecord::new(quality, 1.0)));
            out.push((ScoreType::Volatility, ScoreRecord::new(volatility, 1.0)));
        }

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
