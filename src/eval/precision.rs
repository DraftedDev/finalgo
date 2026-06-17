use crate::consts::TARGET_DEAD_ZONE;
use crate::data::StockData;
use crate::eval::metric::{Metric, MetricInput};
use crate::score::final_score::FinalScore;
use crate::utils::{Value, ValueMap};

/// # Precision Metric
///
/// Computes the precision of LONG, SHORT, and NEUTRAL predictions.
pub struct PrecisionMetric;

impl PrecisionMetric {
    #[inline]
    fn target_decision(target: &StockData, dead_zone: f64) -> &'static str {
        assert!(
            !target.opens.is_empty() && !target.closes.is_empty(),
            "Target data must contain the 7-day window"
        );

        let open = target.opens[0];
        let close = target.closes[target.closes.len() - 1];

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
}

impl Metric for PrecisionMetric {
    fn name(&self) -> String {
        "precision".to_string()
    }

    fn compute(&self, result: &[MetricInput]) -> ValueMap {
        let mut predicted_long = 0usize;
        let mut predicted_short = 0usize;
        let mut predicted_neutral = 0usize;

        let mut correct_long = 0usize;
        let mut correct_short = 0usize;
        let mut correct_neutral = 0usize;

        let mut target_long = 0usize;
        let mut target_short = 0usize;
        let mut target_neutral = 0usize;

        for input in result {
            let decision = input.engine.score::<FinalScore>().decision.as_str();

            let pred = match decision.to_ascii_uppercase().as_str() {
                "LONG" => "LONG",
                "SHORT" => "SHORT",
                "NEUTRAL" => "NEUTRAL",
                other => panic!("Unknown final_decision: {other}"),
            };

            let target = Self::target_decision(&input.target, TARGET_DEAD_ZONE);

            match pred {
                "LONG" => predicted_long += 1,
                "SHORT" => predicted_short += 1,
                "NEUTRAL" => predicted_neutral += 1,
                _ => unreachable!(),
            }

            match target {
                "LONG" => target_long += 1,
                "SHORT" => target_short += 1,
                "NEUTRAL" => target_neutral += 1,
                _ => unreachable!(),
            }

            if pred == target {
                match pred {
                    "LONG" => correct_long += 1,
                    "SHORT" => correct_short += 1,
                    "NEUTRAL" => correct_neutral += 1,
                    _ => unreachable!(),
                }
            }
        }

        let total = result.len();

        let long_precision = if predicted_long > 0 {
            correct_long as f64 / predicted_long as f64
        } else {
            0.0
        };

        let short_precision = if predicted_short > 0 {
            correct_short as f64 / predicted_short as f64
        } else {
            0.0
        };

        let neutral_precision = if predicted_neutral > 0 {
            correct_neutral as f64 / predicted_neutral as f64
        } else {
            0.0
        };

        let overall_precision = if total > 0 {
            (correct_long + correct_short + correct_neutral) as f64 / total as f64
        } else {
            0.0
        };

        ValueMap::new()
            .with("precision_long", Value::Percent(long_precision))
            .with("precision_short", Value::Percent(short_precision))
            .with("precision_neutral", Value::Percent(neutral_precision))
            .with("precision_overall", Value::Percent(overall_precision))
            .with(
                "precision_predicted_long",
                Value::Int(predicted_long as i64),
            )
            .with(
                "precision_predicted_short",
                Value::Int(predicted_short as i64),
            )
            .with(
                "precision_predicted_neutral",
                Value::Int(predicted_neutral as i64),
            )
            .with("precision_correct_long", Value::Int(correct_long as i64))
            .with("precision_correct_short", Value::Int(correct_short as i64))
            .with(
                "precision_correct_neutral",
                Value::Int(correct_neutral as i64),
            )
            .with("precision_target_long", Value::Int(target_long as i64))
            .with("precision_target_short", Value::Int(target_short as i64))
            .with(
                "precision_target_neutral",
                Value::Int(target_neutral as i64),
            )
    }
}
