use crate::data::StockData;
use crate::engine::Engine;
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
    /// The underlying engine of the computation.
    pub engine: Engine,

    /// The target stock data.
    pub target: StockData,
}
