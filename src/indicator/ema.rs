use crate::indicator::Indicator;
use crate::indicator::atr::AvgTrueRange;
use crate::interface::Interface;
use crate::math::norm_atr;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

/// # Exponential Moving Average Indicator.
///
/// ## Purpose
/// - Smooth price trend
/// - Measure directional bias
///
/// ## Math
///
/// ```
/// α = 2 / (period + 1)
///
/// EMA_t = α * price_t + (1 - α) * EMA_{t-1}
/// EMA_DIST_t = norm_atr(price_t - EMA_t)
/// EMA_SLOPE_t = norm_atr_14(EMA_t - EMA_{t-1})
/// ```
pub struct ExpMovAvg<const PERIOD: usize> {
    pub ema: Vec<f64>,
    pub distance: Vec<f64>,
    pub slope: Vec<f64>,
}

impl<const PERIOD: usize> ExpMovAvg<PERIOD> {
    pub fn new() -> Self {
        Self {
            ema: Vec::new(),
            distance: Vec::new(),
            slope: Vec::new(),
        }
    }
}

impl<const PERIOD: usize> Indicator for ExpMovAvg<PERIOD> {
    fn name(&self) -> String {
        format!("ema-{}", PERIOD)
    }

    fn compute(&mut self, int: &Interface) {
        let closes = &int.raw().closes;
        let alpha = 2.0 / (PERIOD as f64 + 1.0);

        self.ema.clear();
        self.distance.clear();
        self.slope.clear();

        self.ema.reserve(closes.len());
        self.distance.reserve(closes.len());
        self.slope.reserve(closes.len());

        let mut ema = closes[0];
        let mut prev_ema = ema;

        for &price in closes.iter() {
            ema = alpha * price + (1.0 - alpha) * ema;

            self.ema.push(ema);

            // Raw bias (will be normalized later)
            self.distance.push(price - ema);

            // Trend slope
            self.slope.push(ema - prev_ema);

            prev_ema = ema;
        }

        let atr = &int.indicator::<AvgTrueRange<14>>().atr;

        self.distance = norm_atr(&self.distance, atr);
        self.slope = norm_atr(&self.slope, atr);
    }

    fn is_computed(&self) -> bool {
        !self.ema.is_empty()
    }

    fn score(&self) -> Vec<ScoreRecord> {
        let mut out = Vec::with_capacity(self.ema.len().saturating_mul(3));

        for i in 2..self.ema.len() {
            let distance = self.distance[i];
            let slope = self.slope[i];

            let prev_slope = self.slope[i - 1];

            let ema_bias = distance.tanh(); // stabilize extremes
            let trend = slope.tanh();

            let direction =
                (ema_bias * 0.65) + (trend * 0.25) + ((ema_bias.signum() * trend).tanh() * 0.10);

            let slope_consistency = 1.0 - (slope - prev_slope).abs().tanh();
            let quality = slope_consistency * 2.0 - 1.0; // map to [-1,1]

            let strength = (ema_bias.abs() * 0.6 + slope.abs() * 0.4).clamp(0.0, 1.0);

            out.push(ScoreRecord::new(
                ScoreType::Direction,
                direction.clamp(-1.0, 1.0),
                0.8,
                1.0,
            ));

            out.push(ScoreRecord::new(
                ScoreType::Quality,
                quality.clamp(-1.0, 1.0),
                0.5,
                1.0,
            ));

            out.push(ScoreRecord::new(ScoreType::Strength, strength, 0.7, 1.0));
        }

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
