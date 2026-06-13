use crate::consts::TARGET_DEAD_ZONE;
use crate::data::StockData;
use crate::eval::metric::{Metric, MetricInput};
use crate::score::final_score::FinalScore;
use crate::utils::{Value, ValueMap};

pub struct PrecisionMetric;

impl PrecisionMetric {
    /// Precision of correctly predicted LONG trades.
    ///
    /// ```text
    /// true_long / predicted_long
    /// ```
    pub const LONG_PRECISION_KEY: &str = "precision_long";

    /// Precision of correctly predicted SHORT trades.
    ///
    /// ```text
    /// true_short / predicted_short
    /// ```
    pub const SHORT_PRECISION_KEY: &str = "precision_short";

    /// Precision of correctly predicted NEUTRAL trades.
    ///
    /// ```text
    /// true_neutral / predicted_neutral
    /// ```
    pub const NEUTRAL_PRECISION_KEY: &str = "precision_neutral";

    /// Overall classification precision across all predictions.
    ///
    /// ```text
    /// (true_long + true_short + true_neutral) / total_predictions
    /// ```
    pub const OVERALL_PRECISION_KEY: &str = "precision_overall";

    /// Number of predictions classified as LONG.
    pub const PRED_LONG_KEY: &str = "precision_predicted_long";

    /// Number of predictions classified as SHORT.
    pub const PRED_SHORT_KEY: &str = "precision_predicted_short";

    /// Number of predictions classified as NEUTRAL.
    pub const PRED_NEUTRAL_KEY: &str = "precision_predicted_neutral";

    /// Number of correct LONG predictions.
    pub const TRUE_LONG_KEY: &str = "precision_true_long";

    /// Number of correct SHORT predictions.
    pub const TRUE_SHORT_KEY: &str = "precision_true_short";

    /// Number of correct NEUTRAL predictions.
    pub const TRUE_NEUTRAL_KEY: &str = "precision_true_neutral";

    #[inline]
    fn target_decision(target: &StockData, dead_zone: f64) -> &'static str {
        assert!(
            !target.opens.is_empty(),
            "Target opens must contain exactly one candle"
        );
        assert!(
            !target.closes.is_empty(),
            "Target closes must contain exactly one candle"
        );

        let open = target.opens[0];
        let close = target.closes[0];

        assert!(open.is_finite(), "Target open must be finite");
        assert!(close.is_finite(), "Target close must be finite");

        let ret = if open.abs() > 1e-12 {
            (close - open) / open
        } else {
            0.0
        };

        if ret > dead_zone {
            "LONG"
        } else if ret < -dead_zone {
            "SHORT"
        } else {
            "NEUTRAL"
        }
    }

    #[inline]
    fn predicted_decision(score: &ValueMap) -> &'static str {
        let decision = score.get(FinalScore::FINAL_SCORE_DECISION_KEY).as_str();

        match decision.to_ascii_uppercase().as_str() {
            "LONG" => "LONG",
            "SHORT" => "SHORT",
            "NEUTRAL" => "NEUTRAL",
            other => panic!("Unknown final_decision: {other}"),
        }
    }
}

impl Metric for PrecisionMetric {
    fn name(&self) -> String {
        "precision".to_string()
    }

    fn compute(&self, result: &[MetricInput]) -> ValueMap {
        let mut predicted_long = 0usize;
        let mut predicted_short = 0usize;
        let mut predicted_neutral = 0usize;

        let mut true_long = 0usize;
        let mut true_short = 0usize;
        let mut true_neutral = 0usize;

        for input in result {
            let pred = Self::predicted_decision(&input.score);
            let target = Self::target_decision(&input.target, TARGET_DEAD_ZONE);

            match pred {
                "LONG" => predicted_long += 1,
                "SHORT" => predicted_short += 1,
                "NEUTRAL" => predicted_neutral += 1,
                _ => unreachable!(),
            }

            if pred == target {
                match pred {
                    "LONG" => true_long += 1,
                    "SHORT" => true_short += 1,
                    "NEUTRAL" => true_neutral += 1,
                    _ => unreachable!(),
                }
            }
        }

        let total = result.len();

        let long_precision = if predicted_long > 0 {
            true_long as f64 / predicted_long as f64
        } else {
            0.0
        };

        let short_precision = if predicted_short > 0 {
            true_short as f64 / predicted_short as f64
        } else {
            0.0
        };

        let neutral_precision = if predicted_neutral > 0 {
            true_neutral as f64 / predicted_neutral as f64
        } else {
            0.0
        };

        let overall_precision = if total > 0 {
            (true_long + true_short + true_neutral) as f64 / total as f64
        } else {
            0.0
        };

        ValueMap::new()
            .with(Self::LONG_PRECISION_KEY, Value::Percent(long_precision))
            .with(Self::SHORT_PRECISION_KEY, Value::Percent(short_precision))
            .with(
                Self::NEUTRAL_PRECISION_KEY,
                Value::Percent(neutral_precision),
            )
            .with(
                Self::OVERALL_PRECISION_KEY,
                Value::Percent(overall_precision),
            )
            .with(Self::PRED_LONG_KEY, predicted_long as i64)
            .with(Self::PRED_SHORT_KEY, predicted_short as i64)
            .with(Self::PRED_NEUTRAL_KEY, predicted_neutral as i64)
            .with(Self::TRUE_LONG_KEY, true_long as i64)
            .with(Self::TRUE_SHORT_KEY, true_short as i64)
            .with(Self::TRUE_NEUTRAL_KEY, true_neutral as i64)
    }
}
