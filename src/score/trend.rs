use crate::engine::Context;
use crate::indicator::ema::ExpMovAvg;
use crate::indicator::er::EfficiencyRatio;
use crate::indicator::roc::RateOfChange;
use crate::indicator::swing::SwingStructure;
use crate::math;
use crate::score::Score;
use crate::utils::ValueMap;
use std::any::Any;

/// # Trend Score
///
/// A score representing the future trend prediction of a stock.
///
/// Requires:
/// - `ExpMovAvg<600>`
/// - `SwingStructure<5, 10>`
/// - `RateOfChange<10>`
/// - `EfficiencyRatio<10, 3>`
pub struct TrendScore {
    /// Final directional trend estimate.
    ///
    /// Represents the aggregated market bias:
    ///
    /// - `+1.0` -> strong bullish trend
    /// - `0.0`  -> neutral / no clear trend
    /// - `-1.0` -> strong bearish trend
    pub direction: f64,

    /// Confidence in the trend estimate.
    ///
    /// Represents how reliable the directional signal is.
    ///
    /// Range:
    /// - `0.0` -> no confidence (noisy / conflicting signals)
    /// - `1.0` -> high confidence (strong alignment across indicators)
    pub confidence: f64,

    computed: bool,
}

impl TrendScore {
    pub const DIRECTION_KEY: &str = "trend_direction";
    pub const CONFIDENCE_KEY: &str = "trend_confidence";

    pub fn new() -> Self {
        Self {
            direction: 0.0,
            confidence: 0.0,
            computed: false,
        }
    }
}

impl Score for TrendScore {
    fn name() -> String {
        "trend".to_string()
    }

    fn compute(&mut self, ctx: Context) -> ValueMap {
        let regime = ctx.regime();

        let ema = ctx.indicator::<ExpMovAvg<600>>();
        let swing = ctx.indicator::<SwingStructure<5, 10>>();
        let roc = ctx.indicator::<RateOfChange<10>>();
        let er = ctx.indicator::<EfficiencyRatio<10, 3>>();

        let close = ctx
            .data()
            .closes
            .last()
            .copied()
            .unwrap_or(1.0)
            .abs()
            .max(1e-12);

        let ema_distance = math::last_finite(&ema.distance).unwrap_or(0.0);
        let ema_slope = math::last_finite(&ema.slope).unwrap_or(0.0);
        let roc_value = math::last_finite(&roc.roc).unwrap_or(0.0);
        let er_value = math::last_finite(&er.smooth).unwrap_or(0.0);
        let structure = math::last_finite(&swing.structure).unwrap_or(0.0);
        let structure_strength = math::last_finite(&swing.structure_strength).unwrap_or(0.0);
        let bos = math::last_finite(&swing.bos).unwrap_or(0.0);
        let choch = math::last_finite(&swing.choch).unwrap_or(0.0);

        let ema_bias = ((ema_distance / close) * 20.0).tanh();
        let ema_momentum = ((ema_slope / close) * 80.0).tanh();

        let roc_bias = (roc_value * 15.0).tanh();

        let structure_bias = structure.clamp(-1.0, 1.0);
        let bos_bias = bos.clamp(-1.0, 1.0);
        let choch_bias = choch.clamp(-1.0, 1.0);

        let regime_trend = regime.trend.clamp(-1.0, 1.0);
        let regime_structure = regime.structure.clamp(-1.0, 1.0);

        let direction = (regime_trend * 0.20
            + regime_structure * 0.15
            + structure_bias * 0.22
            + ema_bias * 0.18
            + ema_momentum * 0.10
            + roc_bias * 0.10
            + bos_bias * 0.03
            + choch_bias * 0.02)
            .clamp(-1.0, 1.0);

        let weighted_abs_sum = regime_trend.abs() * 0.20
            + regime_structure.abs() * 0.15
            + structure_bias.abs() * 0.22
            + ema_bias.abs() * 0.18
            + ema_momentum.abs() * 0.10
            + roc_bias.abs() * 0.10
            + bos_bias.abs() * 0.03
            + choch_bias.abs() * 0.02;

        let consensus = if weighted_abs_sum > 1e-12 {
            (direction.abs() / weighted_abs_sum).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let signal_energy = weighted_abs_sum.clamp(0.0, 1.0);

        let pair_agreement = {
            let a = 1.0 - (regime_trend - structure_bias).abs() * 0.5;
            let b = 1.0 - (structure_bias - ema_bias).abs() * 0.5;
            let c = 1.0 - (ema_bias - roc_bias).abs() * 0.5;
            ((a + b + c) / 3.0).clamp(0.0, 1.0)
        };

        let volatility_penalty = 1.0 - regime.volatility.clamp(0.0, 1.0);

        let confidence = (consensus * signal_energy * 0.35
            + pair_agreement * 0.25
            + er_value.clamp(0.0, 1.0) * 0.20
            + structure_strength.clamp(0.0, 1.0) * 0.10
            + volatility_penalty * 0.10)
            .clamp(0.0, 1.0);

        self.direction = direction;
        self.confidence = confidence;
        self.computed = true;

        ValueMap::new()
            .with(Self::DIRECTION_KEY, self.direction)
            .with(Self::CONFIDENCE_KEY, self.confidence)
    }

    fn is_computed(&self) -> bool {
        self.computed
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
