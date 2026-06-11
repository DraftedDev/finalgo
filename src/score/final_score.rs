use crate::engine::Context;
use crate::score::Score;
use crate::score::participation::ParticipationScore;
use crate::score::quality::QualityScore;
use crate::score::strength::StrengthScore;
use crate::score::trend::TrendScore;
use crate::score::volatility::VolatilityScore;
use crate::utils::ValueMap;
use std::any::Any;

/// # Strength Score
///
/// Measures how clean, stable, and structurally aligned the market is.
///
/// Requires no indicators.
pub struct FinalScore {
    pub score: f64,
    pub confidence: f64,
    pub decision: String,
    computed: bool,
}

impl FinalScore {
    pub const FINAL_SCORE_KEY: &str = "final_score";
    pub const FINAL_SCORE_CONFIDENCE_KEY: &str = "final_confidence";
    pub const FINAL_SCORE_DECISION_KEY: &str = "final_decision";

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
    fn name(&self) -> String {
        "final".to_string()
    }

    fn compute(&mut self, ctx: Context) -> ValueMap {
        let trend = ctx.score::<TrendScore>();
        let strength = ctx.score::<StrengthScore>();
        let volatility = ctx.score::<VolatilityScore>();
        let quality = ctx.score::<QualityScore>();
        let participation = ctx.score::<ParticipationScore>();

        let direction = trend.direction; // [-1, 1]
        let trend_conf = trend.confidence;

        let strength_val = strength.strength; // [0, 1]
        let strength_conf = strength.confidence;

        let vol = volatility.volatility; // [0, 1]
        let vol_conf = volatility.confidence;

        let qual = quality.quality; // [0, 1]
        let qual_conf = quality.confidence;

        let part = participation.participation; // [0, 1]
        let part_conf = participation.confidence;

        let volatility_penalty = 1.0 - (vol * 0.6);
        let structure_boost = (qual * 0.5 + strength_val * 0.5).clamp(0.0, 1.0);
        let participation_boost = part.clamp(0.0, 1.0);

        let raw_score = direction
            * strength_val
            * volatility_penalty
            * structure_boost
            * (0.5 + participation_boost * 0.5);

        let score = raw_score.clamp(-1.0, 1.0);

        let confidence = (trend_conf * 0.35
            + strength_conf * 0.25
            + vol_conf * 0.20
            + qual_conf * 0.10
            + part_conf * 0.10)
            .clamp(0.0, 1.0);

        let decision = if score > 0.25 {
            "LONG"
        } else if score < -0.25 {
            "SHORT"
        } else {
            "NEUTRAL"
        };

        self.score = score;
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
