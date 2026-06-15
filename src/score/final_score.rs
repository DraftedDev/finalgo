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
    /// The key for the final score.
    pub const FINAL_SCORE_KEY: &str = "final_score";

    /// The key for the final score confidence.
    pub const FINAL_SCORE_CONFIDENCE_KEY: &str = "final_confidence";

    /// The key for the final decision.
    pub const FINAL_SCORE_DECISION_KEY: &str = "final_decision";

    /// Creates a new empty [FinalScore] instance.
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

        let env_raw = (strength_val * 0.40) + (qual * 0.40) + (part * 0.20);

        let env_score = (env_raw * vol_factor).clamp(0.0, 1.0);

        let execution_multiplier = env_score.sqrt().clamp(0.1, 1.0);

        let final_score = (direction * execution_multiplier).clamp(-1.0, 1.0);

        let base_confidence = (trend_conf * 0.35
            + strength_conf * 0.25
            + qual_conf * 0.20
            + part_conf * 0.10
            + vol_conf * 0.10)
            .clamp(0.0, 1.0);

        let signal_clarity = final_score.abs();
        let confidence = (base_confidence * 0.75 + signal_clarity * 0.25).clamp(0.0, 1.0);

        let regime_trend = ctx.regime().trend.clamp(-1.0, 1.0);

        let base_long_threshold = if regime_trend > 0.2 {
            0.10
        } else if regime_trend < -0.2 {
            0.25
        } else {
            0.15
        };

        let base_short_threshold = if regime_trend < -0.2 {
            -0.10
        } else if regime_trend > 0.2 {
            -0.25
        } else {
            -0.15
        };

        let long_discount = self.confidence * 0.05;
        let short_discount = self.confidence * 0.05;

        let long_threshold = (base_long_threshold - long_discount).max(0.05);
        let short_threshold = (base_short_threshold + short_discount).min(-0.05);

        self.decision = if final_score > long_threshold {
            "LONG".to_string()
        } else if final_score < short_threshold {
            "SHORT".to_string()
        } else {
            "NEUTRAL".to_string()
        };

        self.score = final_score;
        self.confidence = confidence;
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
