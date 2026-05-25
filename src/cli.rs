use crate::consts::CANDLE_LOOK_BACK;
use crate::eval::{Evaluator, ScoreLoss};
use crate::interface::StockData;
use crate::utils::round_to_two_decimals;
use crate::{interface, utils};
use clap::Parser;

/// Command-line-interface to the finalgo algorithm.
#[derive(Clone, Debug, Parser)]
pub struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Subcommand,
}

impl Cli {
    pub async fn run(&self, args: RunArgs) {
        let data = StockData::fetch_range(args.target, CANDLE_LOOK_BACK, args.ticker).await;
        let mut interface = interface::build(data);

        interface.run();
    }

    pub async fn eval(&self, args: EvalArgs) {
        let mut eval = Evaluator::new();

        let end = utils::parse_naive_date(&args.end);

        let mut t = utils::subtract_naive_date(end, (args.samples as i64 + 1) as usize);

        tracing::info!(
            "Collecting {} samples of {} tickers each...",
            args.samples,
            args.tickers.len()
        );

        for _ in 0..args.samples {
            let next_t = utils::add_naive_date(t, 1);

            for ticker in &args.tickers {
                // Prediction window: [t - LOOKBACK, t]
                let data = StockData::fetch_range(
                    utils::format_naive_date(t),
                    CANDLE_LOOK_BACK,
                    ticker.clone(),
                )
                .await;

                // Target: EXACT next candle ONLY
                let target =
                    StockData::fetch_single(utils::format_naive_date(next_t), ticker.clone()).await;

                eval.add(data, target);
            }

            t = next_t;
        }

        tracing::info!("Evaluating...");

        let losses = eval.eval();
        let aggregate = ScoreLoss::aggregate(&losses);

        tracing::info!("[#############################################]");

        tracing::info!(
            "DIRECTION  || {}",
            round_to_two_decimals(aggregate.direction),
        );

        tracing::info!("QUALITY    || {}", round_to_two_decimals(aggregate.quality),);

        tracing::info!(
            "STRENGTH   || {}",
            round_to_two_decimals(aggregate.strength),
        );

        tracing::info!(
            "VOLATILITY || {}",
            round_to_two_decimals(aggregate.volatility),
        );

        tracing::info!("FINAL      || {}", round_to_two_decimals(aggregate.signal),);

        tracing::info!("TOTAL      || {}", round_to_two_decimals(aggregate.total()),);

        tracing::info!("[#############################################]");
    }
}

/// Subcommands for the finalgo interface.
#[derive(Clone, Debug, Parser)]
pub enum Subcommand {
    /// Run the interface.
    Run(RunArgs),
    /// Evaluate the algorithm with test data.
    Eval(EvalArgs),
}

/// Arguments for the run command.
#[derive(Clone, Debug, Parser)]
pub struct RunArgs {
    /// The target date to predict for.
    pub target: String,
    /// The ticker to use.
    pub ticker: String,
}

/// Arguments for the eval command.
#[derive(Clone, Debug, Parser)]
pub struct EvalArgs {
    /// The end date to use.
    pub end: String,
    /// The sample count to use.
    pub samples: usize,
    /// The ticker to use.
    pub tickers: Vec<String>,
}
