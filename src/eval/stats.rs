use crate::eval::metric::{Metric, MetricInput};
use crate::utils::{Value, ValueMap};

use std::collections::HashMap;

pub struct StatsMetric;

impl Metric for StatsMetric {
    fn name(&self) -> String {
        "stats".to_string()
    }

    fn compute(&self, result: &[MetricInput]) -> ValueMap {
        let mut map: HashMap<String, Stats> = HashMap::new();

        for input in result {
            for (key, value) in input.score.iter() {
                let v = match value {
                    Value::Float(f) => *f,
                    Value::Percent(p) => *p,
                    Value::Int(i) => *i as f64,
                    _ => continue,
                };

                map.entry(key.clone()).or_insert_with(Stats::new).push(v);
            }
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
