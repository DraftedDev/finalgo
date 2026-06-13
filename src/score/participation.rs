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
    fn name() -> String {
        "participation".to_string()
    }

    fn compute(&mut self, ctx: Context) -> ValueMap {
        let regime = ctx.regime();
        let rvol = ctx.indicator::<RelativeVolume<20>>();

        let rvol_mean = math::last_finite_mean(&rvol.values, 3).unwrap_or(1.0);
        let rvol_signal = (rvol_mean - 1.0).clamp(-1.0, 1.0);
        let rvol_participation = 0.5 + 0.5 * rvol_signal;

        let regime_participation = regime.participation.clamp(0.0, 1.0);

        let context_mod = (1.0 + 0.10 * regime.trend.abs() + 0.05 * regime.structure.abs()
            - 0.10 * regime.volatility)
            .clamp(0.75, 1.15);

        let participation = (0.65 * rvol_participation + 0.35 * regime_participation) * context_mod;

        let participation = participation.clamp(0.0, 1.0);

        let agreement = 1.0 - (rvol_participation - regime_participation).abs();

        let stability = (1.0 - regime.volatility).clamp(0.0, 1.0);

        let confidence = (0.7 * agreement + 0.3 * stability).clamp(0.0, 1.0);

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
