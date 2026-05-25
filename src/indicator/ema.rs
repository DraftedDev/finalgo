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

    fn score(&self) -> Vec<(ScoreType, ScoreRecord)> {
        let mut out = Vec::new();

        let distance = self.distance.last().copied().unwrap_or(0.0);
        let slope = self.slope.last().copied().unwrap_or(0.0);

        if !distance.is_finite() || !slope.is_finite() {
            return out;
        }

        let direction = (distance * 0.65 + slope * 0.35).tanh().clamp(-1.0, 1.0);
        out.push((ScoreType::Direction, ScoreRecord::new(direction, 0.90)));

        let strength_raw = distance.abs() * 0.55 + slope.abs() * 0.45;
        let strength = strength_raw.tanh().clamp(0.0, 1.0);
        out.push((ScoreType::Strength, ScoreRecord::new(strength, 0.75)));

        let aligned = distance.signum() == slope.signum();

        let quality = if aligned {
            (distance.abs().min(1.0) + slope.abs().min(1.0)) * 0.5
        } else {
            -((distance.abs().min(1.0) + slope.abs().min(1.0)) * 0.5)
        };

        out.push((
            ScoreType::Quality,
            ScoreRecord::new(quality.clamp(-1.0, 1.0), 0.60),
        ));

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
