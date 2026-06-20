use crate::data::StockData;
use crate::eval::loss::LossMetric;
use crate::eval::metric::{Metric, MetricInput};
use crate::eval::precision::PrecisionMetric;
use crate::eval::profit::ProfitLossMetric;
use crate::eval::stats::StatsMetric;
use crate::utils::{FastMap, ValueMap};
use crate::{engine, math, utils};
use std::fmt::{Display, Formatter};
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
    pub fn eval(&self, samples: Vec<(StockData, StockData)>) -> ValueMap {
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

    /// Ranks the tickers based on the computed metrics.
    ///
    /// Returns a sorted vector of [EvalRank] instances.
    pub fn rank(&self, samples: Vec<(String, Vec<(StockData, StockData)>)>) -> Vec<EvalRank> {
        let mut results = samples
            .into_iter()
            .map(|(ticker, data)| {
                let total_trades = data.len() as f64;

                let eval = self.eval(data);

                let alpha_score = eval.get("pnl_alpha_score").as_float().unwrap();
                let trades_taken = eval.get("pnl_trades_taken").as_int().unwrap() as f64;
                let profit_factor = eval.get("pnl_profit_factor").as_float().unwrap();
                let win_rate = eval.get("pnl_win_rate").as_percent().unwrap();

                EvalRank {
                    rank: 0,
                    ticker,
                    alpha_score,
                    profit_factor,
                    trades_taken: (trades_taken / total_trades) * 100.0,
                    win_rate,
                }
            })
            .collect::<Vec<_>>();

        results.sort_by(|a, b| b.alpha_score.partial_cmp(&a.alpha_score).unwrap());

        for (i, r) in results.iter_mut().enumerate() {
            r.rank = i + 1;
        }

        results
    }
}

/// The evaluation rank of a ticker run.
#[derive(Debug, Clone)]
pub struct EvalRank {
    /// The rank of the ticker in the ranking list.
    pub rank: usize,
    /// The ticker symbol.
    pub ticker: String,
    /// The alpha score computed by the [ProfitLossMetric].
    pub alpha_score: f64,
    /// The profit factor computed by the [ProfitLossMetric].
    pub profit_factor: f64,
    /// The trades taken computed by the [ProfitLossMetric].
    pub trades_taken: f64,
    /// The win rate computed by the [ProfitLossMetric].
    pub win_rate: f64,
}

impl Display for EvalRank {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "   {}: '{}'", self.rank, self.ticker)?;
        writeln!(
            f,
            "      Alpha Score: {}",
            math::round_to(self.alpha_score, 2)
        )?;
        writeln!(
            f,
            "      Profit Factor: {}",
            math::round_to(self.profit_factor, 2)
        )?;
        writeln!(
            f,
            "      Trades Taken: {}%",
            math::round_to(self.trades_taken, 2)
        )?;
        writeln!(f, "      Win Rate: {}%", math::round_to(self.win_rate, 2))?;

        Ok(())
    }
}
