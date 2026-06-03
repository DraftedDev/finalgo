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

        self.ema.reserve(closes.len());
        self.distance.reserve(closes.len());
        self.slope.reserve(closes.len());

        let mut ema = closes[0];
        let mut prev_ema = ema;

        for &price in closes.iter() {
            ema = alpha * price + (1.0 - alpha) * ema;

            // EMA line
            self.ema.push(ema);

            // Distance (bias)
            self.distance.push(price - ema);

            // Slope (trend direction)
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
        let mut out = Vec::with_capacity(self.ema.len());

        for i in 1..self.ema.len() {
            let distance = self.distance[i];
            let slope = self.slope[i];

            let direction = distance * 0.7 + slope * 0.3;
            let quality = -slope.abs();
            let strength = distance.abs().clamp(0.0, 1.0);

            out.push(ScoreRecord::new(
                ScoreType::Direction,
                direction.clamp(-1.0, 1.0),
                0.6,
                1.0,
            ));

            out.push(ScoreRecord::new(
                ScoreType::Quality,
                quality.clamp(-1.0, 1.0),
                0.4,
                1.0,
            ));

            out.push(ScoreRecord::new(
                ScoreType::Strength,
                strength.clamp(0.0, 1.0),
                0.7,
                1.0,
            ));
        }

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
