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

    #[inline]
    fn centered(x: f64) -> f64 {
        x - 0.5
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

        let direction = trend.direction.clamp(-1.0, 1.0);

        let strength_val = strength.strength.clamp(0.0, 1.0);
        let strength_conf = strength.confidence.clamp(0.0, 1.0);

        let vol = volatility.volatility.clamp(0.0, 1.0);

        let qual = quality.quality.clamp(0.0, 1.0);
        let qual_conf = quality.confidence.clamp(0.0, 1.0);

        let part = participation.participation.clamp(0.0, 1.0);
        let part_conf = participation.confidence.clamp(0.0, 1.0);

        let trend_conf = trend.confidence.clamp(0.0, 1.0);

        let gain = 1.0
            + 0.35 * Self::centered(strength_val)
            + 0.25 * Self::centered(qual)
            + 0.15 * Self::centered(part)
            - 0.30 * Self::centered(vol);

        let gain = gain.clamp(0.60, 1.60);

        let score = (direction * gain).clamp(-1.0, 1.0);

        let confidence = (trend_conf * 0.30
            + strength_conf * 0.20
            + qual_conf * 0.20
            + part_conf * 0.15
            + (1.0 - vol) * 0.15)
            .clamp(0.0, 1.0);

        let decision = if score >= 0.18 {
            "LONG"
        } else if score <= -0.18 {
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
