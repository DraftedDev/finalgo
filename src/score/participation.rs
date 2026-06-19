use crate::engine::Context;
use crate::indicator::regime::MarketRegime;
use crate::indicator::rvol::RelativeVolume;
use crate::score::Score;
use std::any::Any;

/// # Participation Score
///
/// A regime-aware market participation score.
///
/// Requires:
/// - `MarketRegime`
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
    /// Creates a new [ParticipationScore].
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

    fn compute(&mut self, ctx: Context) {
        let data = ctx.data();
        let len = data.closes.len();
        if len == 0 {
            self.computed = true;
            return;
        }
        let last_idx = len - 1;

        let regime = ctx.indicator::<MarketRegime>();
        let rvol = ctx.indicator::<RelativeVolume<20>>();

        let regime_participation = regime
            .participation
            .get(last_idx)
            .copied()
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);
        let regime_trend = regime.trend.get(last_idx).copied().unwrap_or(0.0);
        let regime_structure = regime.structure.get(last_idx).copied().unwrap_or(0.0);
        let regime_volatility = regime
            .volatility
            .get(last_idx)
            .copied()
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);

        let rvol_0 = rvol.values.get(last_idx).copied().unwrap_or(1.0);
        let rvol_1 = rvol
            .values
            .get(last_idx.saturating_sub(1))
            .copied()
            .unwrap_or(rvol_0);
        let rvol_2 = rvol
            .values
            .get(last_idx.saturating_sub(2))
            .copied()
            .unwrap_or(rvol_1);

        let rvol_mean_raw = (rvol_0 + rvol_1 + rvol_2) / 3.0;
        let rvol_mean = if rvol_mean_raw.is_finite() {
            rvol_mean_raw
        } else {
            1.0
        };

        let rvol_signal = (rvol_mean - 1.0).clamp(-1.0, 1.0);
        let rvol_participation = 0.5 + 0.5 * rvol_signal;

        let context_mod = (1.0 + 0.10 * regime_trend.abs() + 0.05 * regime_structure.abs()
            - 0.10 * regime_volatility)
            .clamp(0.75, 1.15);

        let participation = (0.65 * rvol_participation + 0.35 * regime_participation) * context_mod;
        let participation = participation.clamp(0.0, 1.0);

        let agreement = 1.0 - (rvol_participation - regime_participation).abs();
        let stability = (1.0 - regime_volatility).clamp(0.0, 1.0);

        let confidence = (0.7 * agreement + 0.3 * stability).clamp(0.0, 1.0);

        self.participation = participation;
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
