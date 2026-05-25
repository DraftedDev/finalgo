use crate::consts::CANDLE_LOOK_BACK;
use crate::eval::{Evaluator, ScoreLoss};
use crate::interface::StockData;
use crate::utils::round_to_two_decimals;
use crate::{interface, utils};
use clap::Parser;
use tokio::task::JoinSet;

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

        let mut t = utils::subtract_naive_date(end, (args.samples as i64 + 5) as usize);

        tracing::info!(
            "Collecting {} samples of {} tickers each...",
            args.samples,
            args.tickers.len()
        );

        let mut data = Vec::with_capacity(args.samples);

        for i in 1..=args.samples {
            let target_end = utils::add_naive_date(t, 4);

            for ticker in &args.tickers {
                data.push((i, t, target_end, ticker.clone()));
            }

            // move forward by one trading day
            t = utils::add_naive_date(t, 1);
        }

        let mut fetched = JoinSet::from_iter(data.into_iter().map(
            |(i, t, target_end, ticker)| async move {
                let data = StockData::fetch_range(
                    utils::format_naive_date(t),
                    CANDLE_LOOK_BACK,
                    ticker.clone(),
                )
                .await;

                let target =
                    StockData::fetch_range(utils::format_naive_date(target_end), 5, ticker.clone())
                        .await;

                tracing::info!("Fetched sample {i}!");

                (data, target)
            },
        ));

        loop {
            if fetched.is_empty() {
                break;
            }

            let (predict, target) = fetched
                .join_next()
                .await
                .expect("Failed to join task")
                .expect("Failed to fetch data");

            eval.add(predict, target);
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
