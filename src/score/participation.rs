use crate::engine::Context;
use crate::indicator::rvol::RelativeVolume;
use crate::math;
use crate::score::Score;
use crate::utils::ValueMap;
use std::any::Any;

/// # Participation Score
///
/// A regime-aware market participation score.
///
/// Requires:
/// - `RelativeVolume<20>`
pub struct ParticipationScore {
    /// Final participation score.
    ///
    /// Represents overall market activity strength.
    ///
    /// Range:
    /// - `0.0` -> extremely inactive / low participation market
    /// - `0.5` -> normal participation environment
    /// - `1.0` -> highly active / high participation market
    pub participation: f64,

    /// Confidence in the participation estimate.
    ///
    /// Measures how reliable the participation signal is.
    ///
    /// Range:
    /// - `0.0` -> weak or unclear participation signal
    /// - `1.0` -> strong, well-confirmed participation conditions
    ///
    /// High confidence usually indicates:
    /// - clear volume expansion or contraction
    /// - agreement between volume and regime structure
    pub confidence: f64,

    computed: bool,
}

impl ParticipationScore {
    pub const PARTICIPATION_KEY: &str = "participation";
    pub const CONFIDENCE_KEY: &str = "participation_confidence";

    pub fn new() -> Self {
        Self {
            participation: 0.0,
            confidence: 0.0,
            computed: false,
        }
    }
}

impl Score for ParticipationScore {
    fn name(&self) -> String {
        "participation".to_string()
    }

    fn compute(&mut self, ctx: Context) -> ValueMap {
        let regime = ctx.regime();

        let rvol = ctx.indicator::<RelativeVolume<20>>();

        // Use a short recent average so the score is less noisy than a single last value.
        let rvol_mean = math::last_finite_mean(&rvol.values, 3).unwrap_or(1.0);

        let rvol_participation = math::sigmoid((rvol_mean - 1.0) * 2.5).clamp(0.0, 1.0);

        let regime_participation = regime.participation.clamp(0.0, 1.0);

        // Small context boost:
        // active trending / structured markets usually have more meaningful participation
        let context_boost = (regime.trend.abs() * 0.15
            + regime.structure.abs() * 0.10
            + (1.0 - regime.volatility).clamp(0.0, 1.0) * 0.10)
            .clamp(0.0, 0.35);

        // Final participation estimate.
        let participation =
            (regime_participation * 0.55 + rvol_participation * 0.45 + context_boost)
                .clamp(0.0, 1.0);

        // Confidence reflects how clearly participation is showing up.
        // Stronger RVOL deviation from baseline means clearer participation.
        let rvol_strength = ((rvol_mean - 1.0).abs() / 2.0).clamp(0.0, 1.0);

        let confidence = (regime_participation * 0.50 + rvol_strength * 0.50).clamp(0.0, 1.0);

        self.participation = participation;
        self.confidence = confidence;
        self.computed = true;

        ValueMap::new()
            .with(Self::PARTICIPATION_KEY, self.participation)
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
