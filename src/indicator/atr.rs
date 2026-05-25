use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::math::z_score;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

/// # Average True Range Indicator
///
/// ## Purpose
/// - Volatility
/// - Risk Momentum
///
/// ## Math
///
/// ```
/// TR = max(
///     high - low,
///     |high - prev_close|,
///     |low - prev_close|
/// );
/// α = 2 / (period + 1)
///
/// ATR_t = α * TR_t + (1 - α) * ATR_{t-1}
/// ATR_Z_t = z_score(ATR_t)
/// ```
pub struct AvgTrueRange<const PERIOD: usize> {
    pub atr: Vec<f64>,
    pub atr_z: Vec<f64>,
}

impl<const PERIOD: usize> AvgTrueRange<PERIOD> {
    pub fn new() -> Self {
        Self {
            atr: Vec::new(),
            atr_z: Vec::new(),
        }
    }
}

impl<const PERIOD: usize> Indicator for AvgTrueRange<PERIOD> {
    fn name(&self) -> String {
        format!("atr-{}", PERIOD)
    }

    fn compute(&mut self, int: &Interface) {
        let data = int.raw();

        let highs = &data.highs;
        let lows = &data.lows;
        let closes = &data.closes;

        let mut tr_values = Vec::with_capacity(closes.len());

        tr_values.push(highs[0] - lows[0]);

        for i in 1..closes.len() {
            let tr = f64::max(
                highs[i] - lows[i],
                f64::max(
                    (highs[i] - closes[i - 1]).abs(),
                    (lows[i] - closes[i - 1]).abs(),
                ),
            );

            tr_values.push(tr);
        }

        let alpha = 2.0 / (PERIOD as f64 + 1.0);

        self.atr = vec![f64::NAN; closes.len()];
        let mut atr = tr_values[0];

        for i in 0..tr_values.len() {
            atr = alpha * tr_values[i] + (1.0 - alpha) * atr;
            self.atr[i] = atr;
        }

        self.atr_z = z_score(&self.atr);
    }

    fn is_computed(&self) -> bool {
        !self.atr.is_empty()
    }

    fn score(&self) -> Vec<(ScoreType, ScoreRecord)> {
        let mut out = Vec::with_capacity(self.atr_z.len());

        for &z in &self.atr_z {
            if !z.is_finite() {
                continue;
            }

            let volatility = (z * 0.8).tanh();

            let weight = (z.abs() / 2.5).clamp(0.15, 1.0);

            out.push((ScoreType::Volatility, ScoreRecord::new(volatility, weight)));
        }

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
