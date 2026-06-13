use crate::engine::Context;
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
    pub const QUALITY_KEY: &str = "quality";
    pub const CONFIDENCE_KEY: &str = "quality_confidence";

    pub fn new() -> Self {
        Self {
            quality: 0.0,
            confidence: 0.0,
            computed: false,
        }
    }

    #[inline]
    fn sign_agreement(trend: f64, structure: f64) -> f64 {
        let trend_mag = trend.abs();
        let structure_mag = structure.abs();

        if trend_mag < 0.15 || structure_mag < 0.15 {
            0.5
        } else if trend.signum() == structure.signum() {
            1.0
        } else {
            0.25
        }
    }
}

impl Score for QualityScore {
    fn name() -> String {
        "quality".to_string()
    }

    fn compute(&mut self, ctx: Context) -> ValueMap {
        let regime = ctx.regime();

        let trend = if regime.trend.is_finite() {
            regime.trend.clamp(-1.0, 1.0)
        } else {
            0.0
        };

        let structure = if regime.structure.is_finite() {
            regime.structure.clamp(-1.0, 1.0)
        } else {
            0.0
        };

        let participation = if regime.participation.is_finite() {
            regime.participation.clamp(0.0, 1.0)
        } else {
            0.0
        };

        let volatility = if regime.volatility.is_finite() {
            regime.volatility.clamp(0.0, 1.0)
        } else {
            1.0
        };

        let trend_mag = trend.abs();
        let structure_mag = structure.abs();
        let stability = 1.0 - volatility;

        let alignment = Self::sign_agreement(trend, structure);

        let base_quality =
            trend_mag * 0.30 + structure_mag * 0.30 + participation * 0.20 + stability * 0.20;

        let quality = (base_quality * (0.70 + 0.30 * alignment)).clamp(0.0, 1.0);

        let confidence_base =
            trend_mag * 0.25 + structure_mag * 0.30 + participation * 0.20 + stability * 0.25;

        let confidence = (confidence_base * alignment).clamp(0.0, 1.0);

        self.quality = quality;
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
