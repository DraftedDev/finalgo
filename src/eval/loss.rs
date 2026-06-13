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
/// Loss metric for trading scores.
pub struct LossMetric;

impl LossMetric {
    pub const DIRECTION_LOSS_KEY: &str = "loss_direction";
    pub const STRENGTH_LOSS_KEY: &str = "loss_strength";
    pub const QUALITY_LOSS_KEY: &str = "loss_quality";
    pub const VOLATILITY_LOSS_KEY: &str = "loss_volatility";
    pub const DECISION_LOSS_KEY: &str = "loss_decision";
    pub const CALIBRATION_LOSS_KEY: &str = "loss_calibration";
    pub const TOTAL_LOSS_KEY: &str = "loss_total";

    #[inline]
    fn signed_loss(pred: f64, target: f64) -> f64 {
        if !pred.is_finite() || !target.is_finite() {
            return 1.0;
        }

        ((pred.clamp(-1.0, 1.0) - target.clamp(-1.0, 1.0)).abs() / 2.0).clamp(0.0, 1.0)
    }

    #[inline]
    fn unsigned_loss(pred: f64, target: f64) -> f64 {
        if !pred.is_finite() || !target.is_finite() {
            return 1.0;
        }

        (pred.clamp(0.0, 1.0) - target.clamp(0.0, 1.0))
            .abs()
            .clamp(0.0, 1.0)
    }

    #[inline]
    fn decision_from_str(s: &str) -> Decision {
        match s.trim().to_ascii_uppercase().as_str() {
            "LONG" => Decision::Long,
            "SHORT" => Decision::Short,
            _ => Decision::Neutral,
        }
    }

    #[inline]
    fn decision_loss(pred: Decision, target: Decision) -> f64 {
        match (pred, target) {
            (Decision::Long, Decision::Long) => 0.0,
            (Decision::Short, Decision::Short) => 0.0,
            (Decision::Neutral, Decision::Neutral) => 0.0,
            (Decision::Neutral, _) => 0.5,
            (_, Decision::Neutral) => 0.5,
            _ => 1.0,
        }
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

        // Direction in [-1, 1]
        let direction = (raw_return * 25.0).tanh().clamp(-1.0, 1.0);

        // Magnitude in [0, 1]
        let strength = (raw_return.abs() * 25.0).tanh().clamp(0.0, 1.0);

        // Volatility in [0, 1] from candle range relative to price
        let range = (high - low).max(0.0);
        let range_ratio = if open.abs() > 1e-12 {
            range / open.abs()
        } else {
            0.0
        };

        let volatility = (1.0 - (-25.0 * range_ratio).exp()).clamp(0.0, 1.0);

        let quality = if range > 1e-12 {
            ((close - open).abs() / range).clamp(0.0, 1.0)
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

        let confidence =
            (0.45 * strength + 0.35 * quality + 0.20 * (1.0 - volatility)).clamp(0.0, 1.0);

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
            let pred_quality_conf = sample.score.get(StrengthScore::CONFIDENCE_KEY).as_float();

            let pred_volatility = sample.score.get(VolatilityScore::VOLATILITY_KEY).as_float();
            let pred_volatility_conf = sample.score.get(VolatilityScore::CONFIDENCE_KEY).as_float();

            let pred_final_confidence = sample
                .score
                .get(FinalScore::FINAL_SCORE_CONFIDENCE_KEY)
                .as_float();
            let pred_decision = Self::decision_from_str(
                sample
                    .score
                    .get(FinalScore::FINAL_SCORE_DECISION_KEY)
                    .as_str(),
            );

            let d_loss = Self::signed_loss(pred_direction, target.direction);
            let s_loss = Self::unsigned_loss(pred_strength, target.strength);
            let q_loss = Self::unsigned_loss(pred_quality, target.quality);
            let v_loss = Self::unsigned_loss(pred_volatility, target.volatility);
            let dec_loss = Self::decision_loss(pred_decision, target.decision);

            let target_confidence = target.confidence;
            let c_loss = (pred_final_confidence - target_confidence)
                .abs()
                .clamp(0.0, 1.0);

            let _component_confidence_hint = (
                pred_direction_conf,
                pred_strength_conf,
                pred_quality_conf,
                pred_volatility_conf,
            );

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

        // Weighted total loss.
        let total_loss = (direction_loss * 0.30
            + strength_loss * 0.20
            + quality_loss * 0.15
            + volatility_loss * 0.10
            + decision_loss * 0.15
            + calibration_loss * 0.10)
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
