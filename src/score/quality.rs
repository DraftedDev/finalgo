use crate::engine::Context;
use crate::indicator::er::EfficiencyRatio;
use crate::math;
use crate::score::Score;
use crate::utils::ValueMap;
use std::any::Any;

/// # Quality Score
///
/// Measures how clean, stable, and structurally aligned the market is.
///
/// Requires no indicators.
pub struct QualityScore {
    /// Final computed market quality score.
    ///
    /// Represents how "tradable" the current market is.
    ///
    /// Range:
    /// - 0.0 -> noisy, unstable, low-quality market
    /// - 1.0 -> clean, structured, high-quality market
    pub quality: f64,

    /// Confidence in the quality estimate.
    ///
    /// Used to down-weight trades when market conditions are uncertain.
    ///
    /// Range:
    /// - 0.0 -> unreliable regime interpretation
    /// - 1.0 -> highly reliable market structure
    pub confidence: f64,

    pub computed: bool,
}

impl QualityScore {
    pub const QUALITY_KEY: &'static str = "quality";
    pub const CONFIDENCE_KEY: &'static str = "quality_confidence";

    pub fn new() -> Self {
        Self {
            quality: 0.0,
            confidence: 0.0,
            computed: false,
        }
    }
}

impl Score for QualityScore {
    fn name() -> String {
        "quality".to_string()
    }

    fn compute(&mut self, ctx: Context) -> ValueMap {
        let regime = ctx.regime();
        let er = ctx.indicator::<EfficiencyRatio<10, 3>>();
        let er_value = math::last_finite(&er.smooth).unwrap_or(0.0).clamp(0.0, 1.0);

        let trend = regime.trend.clamp(-1.0, 1.0);
        let structure = regime.structure.clamp(-1.0, 1.0);
        let participation = regime.participation.clamp(0.0, 1.0);
        let volatility = regime.volatility.clamp(0.0, 1.0);

        let vol_distance = (volatility - 0.5).abs();
        let vol_quality = (1.0 - vol_distance * 2.0).clamp(0.0, 1.0);

        let trend_mag = trend.abs();
        let structure_mag = structure.abs();

        let alignment = if trend_mag < 0.15 || structure_mag < 0.15 {
            0.5
        } else if trend.signum() == structure.signum() {
            1.0
        } else {
            0.2
        };

        let base_quality =
            (vol_quality * 0.35 + alignment * 0.35 + er_value * 0.20 + participation * 0.10)
                .clamp(0.0, 1.0);

        let signal_strength = (trend_mag + structure_mag) / 2.0;
        let conflict_penalty = if alignment < 0.5 { 0.5 } else { 1.0 };

        let confidence = (signal_strength * 0.40 + vol_quality * 0.30 + er_value * 0.30)
            .clamp(0.0, 1.0)
            * conflict_penalty;

        self.quality = base_quality;
        self.confidence = confidence;
        self.computed = true;

        ValueMap::new()
            .with(Self::QUALITY_KEY, self.quality)
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
