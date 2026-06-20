use crate::engine::Context;
use crate::indicator::regime::MarketRegime;
use crate::indicator::veto::MacroVeto;
use crate::score::Score;
use crate::score::participation::ParticipationScore;
use crate::score::quality::QualityScore;
use crate::score::strength::StrengthScore;
use crate::score::trend::TrendScore;
use crate::score::volatility::VolatilityScore;
use std::any::Any;
use std::fmt::{Display, Formatter};

/// # Final Score
///
/// Aggregates all sub-scores into a single actionable signal and decision.
pub struct FinalScore {
    pub score: f64,
    pub confidence: f64,
    pub decision: Decision,
    computed: bool,
}

impl FinalScore {
    pub fn new() -> Self {
        Self {
            score: 0.0,
            confidence: 0.0,
            decision: Decision::Neutral,
            computed: false,
        }
    }
}

impl Score for FinalScore {
    fn name() -> String {
        "final".to_string()
    }

    fn compute(&mut self, ctx: Context) {
        let data = ctx.data();
        let len = data.closes.len();

        if len == 0 {
            self.computed = true;
            return;
        }

        let last_idx = len - 1;

        let trend = ctx.score::<TrendScore>();
        let strength = ctx.score::<StrengthScore>();
        let volatility = ctx.score::<VolatilityScore>();
        let quality = ctx.score::<QualityScore>();
        let participation = ctx.score::<ParticipationScore>();

        let regime = ctx.indicator::<MarketRegime>();
        let veto = ctx.indicator::<MacroVeto>();

        let regime_trend = regime.trend.get(last_idx).copied().unwrap_or(0.0);
        let veto_shorts = veto.veto_shorts.get(last_idx).copied().unwrap_or(false);
        let veto_longs = veto.veto_longs.get(last_idx).copied().unwrap_or(false);

        let direction = trend.direction.clamp(-1.0, 1.0);
        let strength_val = strength.strength.clamp(0.0, 1.0);
        let qual = quality.quality.clamp(0.0, 1.0);
        let part = participation.participation.clamp(0.0, 1.0);
        let vol = volatility.volatility.clamp(0.0, 1.0);

        let vol_factor = (vol * 1.5).clamp(0.1, 1.0);

        let env_raw = strength_val * 0.40 + qual * 0.40 + part * 0.20;
        let env_score = (env_raw * vol_factor).clamp(0.0, 1.0);
        let execution_multiplier = env_score.sqrt().clamp(0.1, 1.0);

        let mut final_score = (direction * execution_multiplier).clamp(-1.0, 1.0);

        let base_confidence = trend.confidence * 0.35
            + strength.confidence * 0.25
            + quality.confidence * 0.20
            + participation.confidence * 0.10
            + volatility.confidence * 0.10;

        let signal_clarity = final_score.abs();
        let final_confidence = (base_confidence * 0.75 + signal_clarity * 0.25).clamp(0.0, 1.0);

        let (base_long, base_short) = if regime_trend > 0.2 {
            (0.10, -0.25)
        } else if regime_trend < -0.2 {
            (0.25, -0.10)
        } else {
            (0.15, -0.15)
        };

        let discount = final_confidence * 0.05;
        let long_threshold = (base_long - discount).max(0.05);
        let short_threshold = (base_short + discount).min(-0.05);

        if (veto_shorts && final_score < 0.0) || (veto_longs && final_score > 0.0) {
            final_score = 0.0;
        }

        let decision = if final_score > long_threshold {
            Decision::Long
        } else if final_score < short_threshold {
            Decision::Short
        } else {
            Decision::Neutral
        };

        self.score = final_score;
        self.confidence = final_confidence;
        self.decision = decision;
        self.computed = true;
    }

    fn is_computed(&self) -> bool {
        self.computed
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Zero-allocation decision enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Long,
    Short,
    Neutral,
}

impl Display for Decision {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Decision::Long => write!(f, "LONG"),
            Decision::Short => write!(f, "SHORT"),
            Decision::Neutral => write!(f, "NEUTRAL"),
        }
    }
}
