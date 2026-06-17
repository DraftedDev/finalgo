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
            "Target StockData must contain the 7-day window"
        );

        let open = target.opens[0];
        let close = target.closes[target.closes.len() - 1];

        let high = target
            .highs
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let low = target.lows.iter().cloned().fold(f64::INFINITY, f64::min);

        let raw_return = if open.abs() > 1e-12 {
            (close - open) / open
        } else {
            0.0
        };
        let range = (high - low).max(0.0);
        let body = (close - open).abs();

        let direction = (raw_return * 50.0).tanh().clamp(-1.0, 1.0);
        let strength = (raw_return.abs() * 50.0).tanh().clamp(0.0, 1.0);

        let mut daily_vol_sum = 0.0;
        let days = target.highs.len().max(1);
        for i in 0..days {
            let open = target.opens[i].abs().max(1e-12);
            let range = (target.highs[i] - target.lows[i]).max(0.0);
            daily_vol_sum += range / open;
        }
        let avg_daily_range = daily_vol_sum / days as f64;

        let volatility = (avg_daily_range * 66.0).tanh().clamp(0.0, 1.0);

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
        let mut direction_loss = 0.0;
        let mut strength_loss = 0.0;
        let mut quality_loss = 0.0;
        let mut volatility_loss = 0.0;
        let mut decision_loss = 0.0;
        let mut calibration_loss = 0.0;

        for sample in result {
            let target = Self::target_from_stock(&sample.target);

            let trend = sample.engine.score::<TrendScore>();
            let strength = sample.engine.score::<StrengthScore>();
            let quality = sample.engine.score::<QualityScore>();
            let volatility = sample.engine.score::<VolatilityScore>();
            let final_score = sample.engine.score::<FinalScore>();

            let pred_decision = match final_score.decision.trim().to_ascii_uppercase().as_str() {
                "LONG" => Decision::Long,
                "SHORT" => Decision::Short,
                _ => Decision::Neutral,
            };

            let d_loss =
                Self::weighted_signed_loss(trend.direction, target.direction, trend.confidence);

            let s_loss = Self::weighted_unsigned_loss(
                strength.strength,
                target.strength,
                strength.confidence,
            );

            let q_loss =
                Self::weighted_unsigned_loss(quality.quality, target.quality, quality.confidence);

            let v_loss = Self::weighted_unsigned_loss(
                volatility.volatility,
                target.volatility,
                volatility.confidence,
            );

            let dec_loss = Self::decision_loss(pred_decision, target.decision, final_score.score);

            let c_loss = Self::unsigned_loss(final_score.confidence, target.confidence);

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
            .with("loss_direction", direction_loss)
            .with("loss_strength", strength_loss)
            .with("loss_quality", quality_loss)
            .with("loss_volatility", volatility_loss)
            .with("loss_decision", decision_loss)
            .with("loss_calibration", calibration_loss)
            .with("loss_total", total_loss)
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
