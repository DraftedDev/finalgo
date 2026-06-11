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
    fn name(&self) -> String {
        "trend".to_string()
    }

    fn compute(&mut self, ctx: Context) -> ValueMap {
        let regime = ctx.regime();

        let ema = ctx.indicator::<ExpMovAvg<600>>();
        let swing = ctx.indicator::<SwingStructure<5, 10>>();
        let roc = ctx.indicator::<RateOfChange<10>>();
        let er = ctx.indicator::<EfficiencyRatio<10, 3>>();

        let ema_distance = math::last_finite(&ema.distance).unwrap_or(0.0);
        let ema_slope = math::last_finite(&ema.slope).unwrap_or(0.0);
        let roc_value = math::last_finite(&roc.roc).unwrap_or(0.0);
        let er_value = math::last_finite(&er.smooth).unwrap_or(0.0);
        let structure = math::last_finite(&swing.structure).unwrap_or(0.0);
        let structure_strength = math::last_finite(&swing.structure_strength).unwrap_or(0.0);
        let bos = math::last_finite(&swing.bos).unwrap_or(0.0);
        let choch = math::last_finite(&swing.choch).unwrap_or(0.0);

        let ema_bias = (ema_distance * 25.0).tanh();
        let ema_momentum = (ema_slope * 50.0).tanh();
        let roc_bias = (roc_value * 15.0).tanh();

        let structure_bias = structure.clamp(-1.0, 1.0);

        // BOS / CHoCH are event-like, so treat them as boosters.
        let bos_bias = bos.clamp(-1.0, 1.0);
        let choch_bias = choch.clamp(-1.0, 1.0);

        // Combine into a raw trend direction.
        //
        // Direction:
        // - positive => bullish
        // - negative => bearish
        let direction = (regime.trend * 0.25
            + regime.structure * 0.15
            + structure_bias * 0.20
            + ema_bias * 0.15
            + ema_momentum * 0.10
            + roc_bias * 0.10
            + bos_bias * 0.03
            + choch_bias * 0.02)
            .clamp(-1.0, 1.0);

        // Confidence should be high when:
        // - structure agrees
        // - ER is high
        // - regime trend is strong
        // - volatility is not extreme
        //
        // If volatility is too high, confidence drops a bit.
        let volatility_penalty = 1.0 - regime.volatility.clamp(0.0, 1.0);

        let confidence = (structure_strength.abs() * 0.30
            + regime.trend.abs() * 0.25
            + regime.structure.abs() * 0.15
            + er_value.clamp(0.0, 1.0) * 0.20
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
