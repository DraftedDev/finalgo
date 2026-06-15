use crate::data::StockData;
use crate::utils::ValueMap;

/// A metric for evaluating the performance of the algorithm.
pub trait Metric: 'static {
    /// Returns the name of the metric.
    fn name(&self) -> String;

    /// Computes the metric and returns the results as a [ValueMap].
    fn compute(&self, result: &[MetricInput]) -> ValueMap;
}

/// Input for the metric.
pub struct MetricInput {
    /// The score of the prediction as [ValueMap].
    pub score: ValueMap,

    /// The target stock data.
    pub target: StockData,
}
