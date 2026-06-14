use crate::engine::Context;
use crate::score::Score;
use crate::score::participation::ParticipationScore;
use crate::score::quality::QualityScore;
use crate::score::strength::StrengthScore;
use crate::score::trend::TrendScore;
use crate::score::volatility::VolatilityScore;
use crate::utils::ValueMap;
use std::any::Any;

/// # Final Score
///
/// Aggregates all sub-scores into a single actionable signal and decision.
pub struct FinalScore {
    pub score: f64,
    pub confidence: f64,
    pub decision: String,
    computed: bool,
}

impl FinalScore {
    pub const FINAL_SCORE_KEY: &'static str = "final_score";
    pub const FINAL_SCORE_CONFIDENCE_KEY: &'static str = "final_confidence";
    pub const FINAL_SCORE_DECISION_KEY: &'static str = "final_decision";

    pub fn new() -> Self {
        Self {
            score: 0.0,
            confidence: 0.0,
            decision: String::new(),
            computed: false,
        }
    }
}

impl Score for FinalScore {
    fn name() -> String {
        "final".to_string()
    }

    fn compute(&mut self, ctx: Context) -> ValueMap {
        let trend = ctx.score::<TrendScore>();
        let strength = ctx.score::<StrengthScore>();
        let volatility = ctx.score::<VolatilityScore>();
        let quality = ctx.score::<QualityScore>();
        let participation = ctx.score::<ParticipationScore>();

        let direction = trend.direction.clamp(-1.0, 1.0);

        let strength_val = strength.strength.clamp(0.0, 1.0);
        let qual = quality.quality.clamp(0.0, 1.0);
        let part = participation.participation.clamp(0.0, 1.0);
        let vol = volatility.volatility.clamp(0.0, 1.0);

        let trend_conf = trend.confidence.clamp(0.0, 1.0);
        let strength_conf = strength.confidence.clamp(0.0, 1.0);
        let qual_conf = quality.confidence.clamp(0.0, 1.0);
        let part_conf = participation.confidence.clamp(0.0, 1.0);
        let vol_conf = volatility.confidence.clamp(0.0, 1.0);

        let vol_distance = (vol - 0.5).abs();
        let vol_factor = (1.0 - vol_distance * 2.0).clamp(0.1, 1.0);

        let env_raw = (strength_val * 0.40) + (qual * 0.30) + (part * 0.30);
        let env_score = (env_raw * vol_factor).clamp(0.0, 1.0);

        let execution_multiplier = env_score.sqrt().clamp(0.2, 1.0);

        let final_score = (direction * execution_multiplier).clamp(-1.0, 1.0);

        let base_confidence = (trend_conf * 0.35
            + strength_conf * 0.25
            + qual_conf * 0.20
            + part_conf * 0.10
            + vol_conf * 0.10)
            .clamp(0.0, 1.0);

        let signal_clarity = final_score.abs();
        let confidence = (base_confidence * 0.75 + signal_clarity * 0.25).clamp(0.0, 1.0);

        let base_threshold = 0.25;
        let confidence_discount = confidence * 0.15;
        let dynamic_threshold = (base_threshold - confidence_discount).max(0.08);

        let decision = if final_score > dynamic_threshold {
            "LONG"
        } else if final_score < -dynamic_threshold {
            "SHORT"
        } else {
            "NEUTRAL"
        };

        self.score = final_score;
        self.confidence = confidence;
        self.decision = decision.to_string();
        self.computed = true;

        ValueMap::new()
            .with(Self::FINAL_SCORE_KEY, self.score)
            .with(Self::FINAL_SCORE_CONFIDENCE_KEY, self.confidence)
            .with(Self::FINAL_SCORE_DECISION_KEY, self.decision.clone())
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
