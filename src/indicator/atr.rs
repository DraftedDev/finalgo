use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::math::z_score;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

/// # Average True Range Indicator
///
/// ## Purpose
/// - Volatility
/// - Risk regime
///
/// ## Math
///
/// ```
/// TR = max(
///     high - low,
///     |high - prev_close|,
///     |low - prev_close|
/// );
///
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

        let len = closes.len();
        if len == 0 {
            self.atr.clear();
            self.atr_z.clear();
            return;
        }

        let mut tr_values = Vec::with_capacity(len);
        tr_values.push(highs[0] - lows[0]);

        for i in 1..len {
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

        self.atr = vec![0.0; len];
        let mut atr = tr_values[0];

        for i in 0..len {
            atr = alpha * tr_values[i] + (1.0 - alpha) * atr;
            self.atr[i] = atr;
        }

        self.atr_z = z_score(&self.atr);
    }

    fn is_computed(&self) -> bool {
        !self.atr.is_empty()
    }

    fn score(&self, _: &Interface) -> Vec<ScoreRecord> {
        let mut out = Vec::new();

        let atr_z = match self.atr_z.last() {
            Some(v) if v.is_finite() => *v,
            _ => return out,
        };

        let normalized_vol = (atr_z / 2.0).tanh();

        out.push(ScoreRecord::new(
            ScoreType::Volatility,
            normalized_vol.clamp(-1.0, 1.0),
            0.85,
            0.90,
        ));

        let quality = (-normalized_vol * 0.25).clamp(-1.0, 1.0);

        out.push(ScoreRecord::new(ScoreType::Quality, quality, 0.10, 0.50));

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
