use crate::consts::TARGET_DEAD_ZONE;
use crate::data::StockData;
use crate::eval::metric::{Metric, MetricInput};
use crate::score::final_score::FinalScore;
use crate::score::quality::QualityScore;
use crate::score::strength::StrengthScore;
use crate::score::trend::TrendScore;
use crate::score::volatility::VolatilityScore;
use crate::utils::ValueMap;

/// # Loss Metric
///
/// Evaluates the predictive accuracy of the scoring engine against actual market outcomes.
pub struct LossMetric;

impl LossMetric {
    pub const DIRECTION_LOSS_KEY: &'static str = "loss_direction";
    pub const STRENGTH_LOSS_KEY: &'static str = "loss_strength";
    pub const QUALITY_LOSS_KEY: &'static str = "loss_quality";
    pub const VOLATILITY_LOSS_KEY: &'static str = "loss_volatility";
    pub const DECISION_LOSS_KEY: &'static str = "loss_decision";
    pub const CALIBRATION_LOSS_KEY: &'static str = "loss_calibration";
    pub const TOTAL_LOSS_KEY: &'static str = "loss_total";

    /// Base Mean Absolute Error for [-1.0, 1.0] bounded signals.
    #[inline]
    fn signed_loss(pred: f64, target: f64) -> f64 {
        if !pred.is_finite() || !target.is_finite() {
            return 1.0;
        }
        ((pred.clamp(-1.0, 1.0) - target.clamp(-1.0, 1.0)).abs() / 2.0).clamp(0.0, 1.0)
    }

    /// Base Mean Absolute Error for [0.0, 1.0] bounded signals.
    #[inline]
    fn unsigned_loss(pred: f64, target: f64) -> f64 {
        if !pred.is_finite() || !target.is_finite() {
            return 1.0;
        }
        (pred.clamp(0.0, 1.0) - target.clamp(0.0, 1.0))
            .abs()
            .clamp(0.0, 1.0)
    }

    /// Confidence-Weighted Signed Loss (Proper Scoring Rule)
    /// High confidence errors are penalized much more heavily than low confidence errors.
    #[inline]
    fn weighted_signed_loss(pred: f64, target: f64, pred_conf: f64) -> f64 {
        let base_loss = Self::signed_loss(pred, target);
        let conf_weight = 0.5 + 0.5 * pred_conf.clamp(0.0, 1.0);
        (base_loss * conf_weight).clamp(0.0, 1.0)
    }

    /// Confidence-Weighted Unsigned Loss
    #[inline]
    fn weighted_unsigned_loss(pred: f64, target: f64, pred_conf: f64) -> f64 {
        let base_loss = Self::unsigned_loss(pred, target);
        let conf_weight = 0.5 + 0.5 * pred_conf.clamp(0.0, 1.0);
        (base_loss * conf_weight).clamp(0.0, 1.0)
    }

    /// Categorical decision loss weighted by prediction conviction.
    #[inline]
    fn decision_loss(pred: Decision, target: Decision, pred_conviction: f64) -> f64 {
        let base_loss = match (pred, target) {
            (Decision::Long, Decision::Long)
            | (Decision::Short, Decision::Short)
            | (Decision::Neutral, Decision::Neutral) => 0.0,

            (Decision::Neutral, _) | (_, Decision::Neutral) => 0.5,

            _ => 1.0,
        };

        let conviction_multiplier = 0.5 + (pred_conviction.abs() * 0.5);
        (base_loss * conviction_multiplier).clamp(0.0, 1.0)
    }

    fn target_from_stock(target: &StockData) -> TargetSample {
        assert!(
            !target.opens.is_empty()
                && !target.closes.is_empty()
                && !target.highs.is_empty()
                && !target.lows.is_empty(),
            "Target StockData must contain one future candle"
        );

        let open = target.opens[0];
        let close = target.closes[0];
        let high = target.highs[0];
        let low = target.lows[0];

        let raw_return = if open.abs() > 1e-12 {
            (close - open) / open
        } else {
            0.0
        };
        let range = (high - low).max(0.0);
        let range_ratio = if open.abs() > 1e-12 {
            range / open.abs()
        } else {
            0.0
        };
        let body = (close - open).abs();

        let direction = (raw_return * 50.0).tanh().clamp(-1.0, 1.0);
        let strength = (raw_return.abs() * 50.0).tanh().clamp(0.0, 1.0);

        let volatility = (range_ratio * 33.0).tanh().clamp(0.0, 1.0);

        let quality = if range > 1e-12 {
            (body / range).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let decision = if raw_return > TARGET_DEAD_ZONE {
            Decision::Long
        } else if raw_return < -TARGET_DEAD_ZONE {
            Decision::Short
        } else {
            Decision::Neutral
        };

        let vol_penalty = if volatility > 0.8 && quality < 0.4 {
            0.5
        } else {
            1.0
        };
        let confidence = ((strength * 0.5 + quality * 0.5) * vol_penalty).clamp(0.0, 1.0);

        TargetSample {
            direction,
            strength,
            quality,
            volatility,
            decision,
            confidence,
        }
    }
}

impl Metric for LossMetric {
    fn name(&self) -> String {
        "loss".to_string()
    }

    fn compute(&self, result: &[MetricInput]) -> ValueMap {
        if result.is_empty() {
            return ValueMap::new()
                .with(Self::DIRECTION_LOSS_KEY, 0.0)
                .with(Self::STRENGTH_LOSS_KEY, 0.0)
                .with(Self::QUALITY_LOSS_KEY, 0.0)
                .with(Self::VOLATILITY_LOSS_KEY, 0.0)
                .with(Self::DECISION_LOSS_KEY, 0.0)
                .with(Self::CALIBRATION_LOSS_KEY, 0.0)
                .with(Self::TOTAL_LOSS_KEY, 0.0);
        }

        let mut direction_loss = 0.0;
        let mut strength_loss = 0.0;
        let mut quality_loss = 0.0;
        let mut volatility_loss = 0.0;
        let mut decision_loss = 0.0;
        let mut calibration_loss = 0.0;

        for sample in result {
            let target = Self::target_from_stock(&sample.target);

            let pred_direction = sample.score.get(TrendScore::DIRECTION_KEY).as_float();
            let pred_direction_conf = sample.score.get(TrendScore::CONFIDENCE_KEY).as_float();

            let pred_strength = sample.score.get(StrengthScore::STRENGTH_KEY).as_float();
            let pred_strength_conf = sample.score.get(StrengthScore::CONFIDENCE_KEY).as_float();

            let pred_quality = sample.score.get(QualityScore::QUALITY_KEY).as_float();
            let pred_quality_conf = sample.score.get(QualityScore::CONFIDENCE_KEY).as_float();

            let pred_volatility = sample.score.get(VolatilityScore::VOLATILITY_KEY).as_float();
            let pred_volatility_conf = sample.score.get(VolatilityScore::CONFIDENCE_KEY).as_float();

            let pred_final_score = sample.score.get(FinalScore::FINAL_SCORE_KEY).as_float();
            let pred_final_confidence = sample
                .score
                .get(FinalScore::FINAL_SCORE_CONFIDENCE_KEY)
                .as_float();

            let pred_decision_str = sample
                .score
                .get(FinalScore::FINAL_SCORE_DECISION_KEY)
                .as_str();
            let pred_decision = match pred_decision_str.trim().to_ascii_uppercase().as_str() {
                "LONG" => Decision::Long,
                "SHORT" => Decision::Short,
                _ => Decision::Neutral,
            };

            let d_loss =
                Self::weighted_signed_loss(pred_direction, target.direction, pred_direction_conf);
            let s_loss =
                Self::weighted_unsigned_loss(pred_strength, target.strength, pred_strength_conf);
            let q_loss =
                Self::weighted_unsigned_loss(pred_quality, target.quality, pred_quality_conf);
            let v_loss = Self::weighted_unsigned_loss(
                pred_volatility,
                target.volatility,
                pred_volatility_conf,
            );

            let dec_loss = Self::decision_loss(pred_decision, target.decision, pred_final_score);

            let c_loss = Self::unsigned_loss(pred_final_confidence, target.confidence);

            direction_loss += d_loss;
            strength_loss += s_loss;
            quality_loss += q_loss;
            volatility_loss += v_loss;
            decision_loss += dec_loss;
            calibration_loss += c_loss;
        }

        let n = result.len() as f64;

        direction_loss /= n;
        strength_loss /= n;
        quality_loss /= n;
        volatility_loss /= n;
        decision_loss /= n;
        calibration_loss /= n;

        let total_loss = (direction_loss * 0.30
            + decision_loss * 0.25
            + strength_loss * 0.15
            + calibration_loss * 0.15
            + quality_loss * 0.10
            + volatility_loss * 0.05)
            .clamp(0.0, 1.0);

        ValueMap::new()
            .with(Self::DIRECTION_LOSS_KEY, direction_loss)
            .with(Self::STRENGTH_LOSS_KEY, strength_loss)
            .with(Self::QUALITY_LOSS_KEY, quality_loss)
            .with(Self::VOLATILITY_LOSS_KEY, volatility_loss)
            .with(Self::DECISION_LOSS_KEY, decision_loss)
            .with(Self::CALIBRATION_LOSS_KEY, calibration_loss)
            .with(Self::TOTAL_LOSS_KEY, total_loss)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Decision {
    Long,
    Short,
    Neutral,
}

struct TargetSample {
    direction: f64,
    strength: f64,
    quality: f64,
    volatility: f64,
    decision: Decision,
    confidence: f64,
}
