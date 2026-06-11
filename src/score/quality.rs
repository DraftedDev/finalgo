use crate::engine::Context;
use crate::score::Score;
use crate::utils::ValueMap;
use std::any::Any;

/// # Strength Score
///
/// Measures how clean, stable, and structurally aligned the market is.
///
/// Requires no indicators.
pub struct QualityScore {
    pub quality: f64,
    pub confidence: f64,
    computed: bool,
}

impl QualityScore {
    pub const QUALITY_KEY: &str = "quality";
    pub const CONFIDENCE_KEY: &str = "quality_confidence";

    pub fn new() -> Self {
        Self {
            quality: 0.0,
            confidence: 0.0,
            computed: false,
        }
    }
}

impl Score for QualityScore {
    fn name(&self) -> String {
        "quality".to_string()
    }

    fn compute(&mut self, ctx: Context) -> ValueMap {
        let regime = ctx.regime();

        let trend = if regime.trend.is_finite() {
            regime.trend.abs().clamp(0.0, 1.0)
        } else {
            0.0
        };

        let structure = if regime.structure.is_finite() {
            regime.structure.abs().clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Higher when volatility is lower.
        let stability = 1.0 - regime.volatility;

        // Penalize disagreement between trend and structure.
        let alignment = if regime.trend.is_finite()
            && regime.structure.is_finite()
            && trend > 0.2
            && structure > 0.2
        {
            if regime.trend.signum() == regime.structure.signum() {
                1.0
            } else {
                0.75
            }
        } else {
            0.90
        };

        // Clean market = trend + structure + participation + stability.
        let raw_quality =
            (trend * 0.30 + structure * 0.30 + regime.participation * 0.20 + stability * 0.20)
                * alignment;

        // Confidence is similar, but slightly more conservative.
        let raw_confidence =
            (trend * 0.25 + structure * 0.30 + regime.participation * 0.20 + stability * 0.25)
                * alignment;

        self.quality = raw_quality.clamp(0.0, 1.0);
        self.confidence = raw_confidence.clamp(0.0, 1.0);
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
