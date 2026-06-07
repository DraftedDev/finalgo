use crate::consts::{CANDLE_LOOK_BACK, FETCH_CHUNK_SIZE, TARGET_CANDLE_LOOK_BACK};
use crate::data::{DataKey, StockData};
use crate::database::Database;
use crate::eval::{Evaluator, ScoreLoss};
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
        let database = Database::new();

        let data = StockData::fetch(
            &database,
            DataKey {
                end: args.target,
                size: CANDLE_LOOK_BACK,
                ticker: args.ticker,
            },
        )
        .await;
        let mut interface = interface::build(data);

        interface.run(true);
    }

    pub async fn eval(&self, args: EvalArgs) {
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

        let mut eval = utils::with_progress("Fetching", data.len() as u64, |span| async move {
            let database = Database::new();
            let mut eval = Evaluator::new();

            for chunk in data.chunks(FETCH_CHUNK_SIZE).map(|chunk| chunk.to_vec()) {
                let mut set = JoinSet::new();

                for (t, t_target, ticker) in chunk {
                    let database = database.clone();
                    let span = span.clone();
                    set.spawn(async move {
                        let data = StockData::fetch(
                            &database,
                            DataKey {
                                end: utils::format_naive_date(t),
                                size: CANDLE_LOOK_BACK,
                                ticker: ticker.clone(),
                            },
                        )
                        .await;

                        let target = StockData::fetch(
                            &database,
                            DataKey {
                                end: utils::format_naive_date(t_target),
                                size: TARGET_CANDLE_LOOK_BACK,
                                ticker: ticker.clone(),
                            },
                        )
                        .await;

                        span.pb_inc(1);

                        (data, target)
                    });
                }

                for (predict, target) in set.join_all().await {
                    eval.add(predict, target);
                }
            }

            eval
        })
        .await;

        tracing::info!("Evaluating...");

        let (losses, report) = eval.eval().await;
        let loss = ScoreLoss::aggregate(&losses);

        tracing::info!("[############### LOSS ###############]");
        loss.print();

        tracing::info!("[############### ACCURACY ###############]");
        report.print();
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
