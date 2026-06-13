use crate::data::StockData;
use crate::engine::Engine;
use crate::eval::loss::LossMetric;
use crate::eval::metric::{Metric, MetricInput};
use crate::eval::precision::PrecisionMetric;
use crate::eval::stats::StatsMetric;
use crate::utils::{FastMap, ValueMap};
use crate::{engine, utils};
use tracing_indicatif::span_ext::IndicatifSpanExt;

pub mod loss;
pub mod metric;
pub mod precision;
pub mod stats;

pub fn build(stats: bool) -> Evaluator {
    let mut evaluator = Evaluator::new();

    evaluator.add_metric(PrecisionMetric);
    evaluator.add_metric(LossMetric);

    if stats {
        evaluator.add_metric(StatsMetric);
    }

    evaluator
}

pub struct Evaluator {
    engine: Engine,
    metrics: FastMap<String, Box<dyn Metric>>,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            engine: engine::build(),
            metrics: FastMap::with_capacity_and_hasher(16, Default::default()),
        }
    }

    pub fn add_metric(&mut self, metric: impl Metric) {
        let name = metric.name();

        if self.metrics.contains_key(&name) {
            panic!("Metric already initialized");
        }

        self.metrics.insert(name, Box::new(metric));
    }

    pub fn eval(&mut self, samples: Vec<(StockData, StockData)>) -> ValueMap {
        let inputs = utils::with_progress("Computing", samples.len() as u64, |span| {
            let mut results = Vec::with_capacity(samples.len());

            for (data, target) in samples {
                let score = self.engine.compute(false, &data);
                self.engine.reset();

                results.push(MetricInput { score, target });

                span.pb_inc(1);
            }

            results
        });

        let result = utils::with_progress("Evaluating", inputs.len() as u64, |span| {
            let mut result = ValueMap::new();

            for metric in self.metrics.values() {
                let metric_result = metric.compute(&inputs);
                result.merge(metric_result);

                span.pb_inc(1);
            }

            result
        });

        result
    }
}
