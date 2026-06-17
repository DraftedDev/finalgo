use crate::data::StockData;
use crate::eval::loss::LossMetric;
use crate::eval::metric::{Metric, MetricInput};
use crate::eval::precision::PrecisionMetric;
use crate::eval::profit::ProfitLossMetric;
use crate::eval::stats::StatsMetric;
use crate::utils::{FastMap, ValueMap};
use crate::{engine, utils};
use tracing_indicatif::span_ext::IndicatifSpanExt;

/// Contains the loss metric.
pub mod loss;

/// Contains the metric trait and the input structure.
pub mod metric;

/// Contains the precision metric.
pub mod precision;

/// Contains the profit-loss metric.
pub mod profit;

/// Contains the statistics metric.
pub mod stats;

/// Builds the evaluator with the complete set of metrics.
pub fn build(stats: bool) -> Evaluator {
    let mut evaluator = Evaluator::new();

    evaluator.add_metric(PrecisionMetric);
    evaluator.add_metric(LossMetric);
    evaluator.add_metric(ProfitLossMetric);

    if stats {
        evaluator.add_metric(StatsMetric);
    }

    evaluator
}

/// The evaluator struct for evaluating the engine algorithm.
pub struct Evaluator {
    metrics: FastMap<String, Box<dyn Metric>>,
}

impl Evaluator {
    /// Initializes a new evaluator.
    ///
    /// It's recommended to use the [build] function instead of this constructor.
    pub fn new() -> Self {
        Self {
            metrics: FastMap::with_capacity_and_hasher(16, Default::default()),
        }
    }

    /// Add a metric to the evaluator.
    ///
    /// Metrics must be unique, otherwise a panic will occur.
    pub fn add_metric(&mut self, metric: impl Metric) {
        let name = metric.name();

        if self.metrics.contains_key(&name) {
            panic!("Metric already initialized");
        }

        self.metrics.insert(name, Box::new(metric));
    }

    /// Evaluates the engine algorithm on the given samples.
    pub fn eval(&mut self, samples: Vec<(StockData, StockData)>) -> ValueMap {
        let inputs = utils::with_progress("Computing", samples.len() as u64, |span| {
            let mut results = Vec::with_capacity(samples.len());

            for (data, target) in samples {
                let mut engine = engine::build();

                engine.compute(false, &data);

                results.push(MetricInput { engine, target });

                span.pb_inc(1);
            }

            results
        });

        utils::with_progress("Evaluating", inputs.len() as u64, |span| {
            let mut result = ValueMap::new();

            for metric in self.metrics.values() {
                let metric_result = metric.compute(&inputs);
                result.merge(metric_result);

                span.pb_inc(1);
            }

            result
        })
    }
}
