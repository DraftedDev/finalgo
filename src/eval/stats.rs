use crate::eval::metric::{Metric, MetricInput};
use crate::utils::ValueMap;

use crate::score::final_score::FinalScore;
use crate::score::participation::ParticipationScore;
use crate::score::quality::QualityScore;
use crate::score::strength::StrengthScore;
use crate::score::trend::TrendScore;
use crate::score::volatility::VolatilityScore;
use std::collections::HashMap;

/// Computes statistics for different metrics.
///
/// Extremely useful for development and analysis.
pub struct StatsMetric;

impl Metric for StatsMetric {
    fn name(&self) -> String {
        "stats".to_string()
    }

    fn compute(&self, result: &[MetricInput]) -> ValueMap {
        let mut map: HashMap<String, Stats> = HashMap::new();

        for input in result {
            let trend = input.engine.score::<TrendScore>();
            let quality = input.engine.score::<QualityScore>();
            let strength = input.engine.score::<StrengthScore>();
            let participation = input.engine.score::<ParticipationScore>();
            let volatility = input.engine.score::<VolatilityScore>();
            let final_score = input.engine.score::<FinalScore>();

            map.entry("trend_direction".to_string())
                .or_insert_with(Stats::new)
                .push(trend.direction);

            map.entry("trend_confidence".to_string())
                .or_insert_with(Stats::new)
                .push(trend.confidence);

            map.entry("quality".to_string())
                .or_insert_with(Stats::new)
                .push(quality.quality);

            map.entry("quality_confidence".to_string())
                .or_insert_with(Stats::new)
                .push(quality.confidence);

            map.entry("strength".to_string())
                .or_insert_with(Stats::new)
                .push(strength.strength);

            map.entry("strength_confidence".to_string())
                .or_insert_with(Stats::new)
                .push(strength.confidence);

            map.entry("participation".to_string())
                .or_insert_with(Stats::new)
                .push(participation.participation);

            map.entry("participation_confidence".to_string())
                .or_insert_with(Stats::new)
                .push(participation.confidence);

            map.entry("volatility".to_string())
                .or_insert_with(Stats::new)
                .push(volatility.volatility);

            map.entry("volatility_confidence".to_string())
                .or_insert_with(Stats::new)
                .push(volatility.confidence);

            map.entry("final_score".to_string())
                .or_insert_with(Stats::new)
                .push(final_score.score);

            map.entry("final_confidence".to_string())
                .or_insert_with(Stats::new)
                .push(final_score.confidence);
        }

        let mut out = ValueMap::new();

        for (key, stats) in map {
            out = out
                .with(format!("stats_{key}_min"), stats.min)
                .with(format!("stats_{key}_max"), stats.max)
                .with(format!("stats_{key}_mean"), stats.mean())
        }

        out
    }
}

struct Stats {
    min: f64,
    max: f64,
    sum: f64,
    count: usize,
}

impl Stats {
    fn new() -> Self {
        Self {
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            sum: 0.0,
            count: 0,
        }
    }

    fn push(&mut self, v: f64) {
        self.min = self.min.min(v);
        self.max = self.max.max(v);
        self.sum += v;
        self.count += 1;
    }

    fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }
}
