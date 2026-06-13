use crate::data::StockData;
use crate::utils::ValueMap;

pub trait Metric: 'static {
    fn name(&self) -> String;
    fn compute(&self, result: &[MetricInput]) -> ValueMap;
}

pub struct MetricInput {
    pub score: ValueMap,
    pub target: StockData,
}
