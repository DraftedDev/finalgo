use crate::engine::Context;
use crate::indicator::atr::AvgTrueRange;
use crate::indicator::roc::RateOfChange;
use crate::indicator::rvol::RelativeVolume;
use crate::indicator::swing::SwingStructure;
use crate::math;
use crate::score::Score;
use std::any::Any;

/// # Strength Score
///
/// Represents how strong the current trend is, regardless of direction.
///
/// Requires:
/// - `ExpMovAvg<600>`
/// - `SwingStructure<5, 10>`
/// - `RateOfChange<10>`
/// - `EfficiencyRatio<10, 3>`
pub struct StrengthScore {
    /// Strength of the current trend.
    ///
    /// Range:
    /// - 0.0 -> no meaningful trend / weak or choppy movement
    /// - 1.0 -> very strong, sustained directional trend
    pub strength: f64,

    /// Confidence in the strength estimate.
    ///
    /// Range:
    /// - 0.0 -> unreliable / conflicting signals
    /// - 1.0 -> highly consistent and trustworthy trend conditions
    pub confidence: f64,

    computed: bool,
}

impl StrengthScore {
    /// Creates a new [StrengthScore] instance.
    pub fn new() -> Self {
        Self {
            strength: 0.0,
            confidence: 0.0,
            computed: false,
        }
    }
}

impl Score for StrengthScore {
    fn name() -> String {
        "strength".to_string()
    }

    fn compute(&mut self, ctx: Context) {
        let roc = ctx.indicator::<RateOfChange<10>>();
        let atr = ctx.indicator::<AvgTrueRange<14>>();
        let swing = ctx.indicator::<SwingStructure<5, 10>>();
        let rvol = ctx.indicator::<RelativeVolume<20>>();
        let regime = ctx.regime();

        let roc_mag = math::last_finite(&roc.roc_abs).unwrap_or(0.0);
        let roc_strength = (roc_mag * 40.0).tanh();

        let atr_norm = math::last_finite(&atr.norm_atr).unwrap_or(0.0);
        let atr_strength = (atr_norm * 50.0).tanh();

        let structure_str = math::last_finite(&swing.structure_strength)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);

        let rvol_mean = math::last_finite_mean(&rvol.values, 3).unwrap_or(1.0);
        let vol_participation = ((rvol_mean - 0.5) / 1.5).clamp(0.0, 1.0);

        let strength = (roc_strength * 0.40
            + atr_strength * 0.30
            + structure_str * 0.20
            + vol_participation * 0.10)
            .clamp(0.0, 1.0);

        let signal_stability = 1.0 - (regime.volatility - 0.5).abs() * 2.0;
        let confidence = (roc_strength * 0.30
            + structure_str * 0.30
            + vol_participation * 0.20
            + signal_stability.clamp(0.0, 1.0) * 0.20)
            .clamp(0.0, 1.0);

        self.strength = strength;
        self.confidence = confidence;
        self.computed = true;
    }

    fn is_computed(&self) -> bool {
        self.computed
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
