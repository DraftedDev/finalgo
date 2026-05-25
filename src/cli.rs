use crate::consts::CANDLE_LOOK_BACK;
use crate::eval::{Evaluator, ScoreLoss};
use crate::interface::StockData;
use crate::utils::round_to_two_decimals;
use crate::{interface, utils};
use clap::Parser;
use tokio::task::JoinSet;
use tracing_indicatif::span_ext::IndicatifSpanExt;

/// Command-line-interface to the finalgo algorithm.
#[derive(Clone, Debug, Parser)]
pub struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Subcommand,
}

impl Cli {
    pub async fn run(&self, args: RunArgs) {
        let data = StockData::fetch(args.target, CANDLE_LOOK_BACK, args.ticker).await;
        let mut interface = interface::build(data);

        interface.run(true);
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

        for _ in 1..=args.samples {
            let target_end = utils::add_naive_date(t, 4);

            for ticker in &args.tickers {
                data.push((t, target_end, ticker.clone()));
            }

            // move forward by one trading day
            t = utils::add_naive_date(t, 1);
        }

        let fetched = utils::with_progress("Fetching", data.len() as u64, |span| async move {
            let mut set = JoinSet::new();

            for (t, t_target, ticker) in data {
                let span = span.clone();
                set.spawn(async move {
                    let data = StockData::fetch(
                        utils::format_naive_date(t),
                        CANDLE_LOOK_BACK,
                        ticker.clone(),
                    )
                    .await;

                    let target =
                        StockData::fetch(utils::format_naive_date(t_target), 5, ticker.clone())
                            .await;

                    span.pb_inc(1);

                    (data, target)
                });
            }

            set.join_all().await
        })
        .await;

        for (predict, target) in fetched {
            eval.add(predict, target);
        }

        tracing::info!("Evaluating...");

        let losses = eval.eval().await;
        let aggregate = ScoreLoss::aggregate(&losses);

        tracing::info!("[####################]");

        tracing::info!(
            "DIRECTION  || {}",
            round_to_two_decimals(aggregate.direction),
        );

        tracing::info!("QUALITY    || {}", round_to_two_decimals(aggregate.quality));

        tracing::info!(
            "STRENGTH   || {}",
            round_to_two_decimals(aggregate.strength),
        );

        tracing::info!(
            "VOLATILITY || {}",
            round_to_two_decimals(aggregate.volatility),
        );

        tracing::info!("FINAL      || {}", round_to_two_decimals(aggregate.signal));

        tracing::info!("TOTAL      || {}", round_to_two_decimals(aggregate.total()));

        tracing::info!("[####################]");
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
