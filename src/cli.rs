use crate::consts::{CANDLE_LOOK_BACK, FETCH_CHUNK_SIZE, TARGET_HORIZON};
use crate::data::{DataKey, StockData};
use crate::database::Database;
use crate::eval::profit::{STOP_LOSS, TAKE_PROFIT};
use crate::score::final_score::FinalScore;
use crate::{engine, eval, utils};
use clap::Parser;
use std::sync::Arc;
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
    /// Runs the finalgo algorithm with given arguments.
    pub async fn run(&self, args: RunArgs) {
        let target_end_date =
            utils::add_naive_date(utils::parse_naive_date(&args.target), TARGET_HORIZON);
        let target_end_str = utils::format_naive_date(target_end_date);

        let data = StockData::fetch_alpaca(
            &utils::client(),
            &DataKey {
                end: args.target.clone(),
                size: CANDLE_LOOK_BACK,
                ticker: args.ticker.clone(),
            },
        )
        .await;

        let mut engine = engine::build();
        let score = engine.compute(true, &data);

        if args.trade {
            let decision = score.get(FinalScore::FINAL_SCORE_DECISION_KEY).as_str();
            let entry_price = data.closes.last().copied().unwrap_or(0.0);

            tracing::info!("[######################### TRADE #########################]");
            tracing::info!("Ticker: {}", args.ticker);
            tracing::info!("Decision: {}", decision);
            tracing::info!("Predicted Target Date: {}", target_end_str); // <--- NEW LINE

            if decision == "LONG" {
                let sl = entry_price * (1.0 - STOP_LOSS);
                let tp = entry_price * (1.0 + TAKE_PROFIT);

                tracing::info!("Entry: ${:.2}", entry_price);
                tracing::info!("Stop Loss: ${:.2} (-{:.0}%)", sl, STOP_LOSS * 100.0);
                tracing::info!("Take Profit: ${:.2} (+{:.0}%)", tp, TAKE_PROFIT * 100.0);
            } else if decision == "SHORT" {
                let sl = entry_price * (1.0 + STOP_LOSS);
                let tp = entry_price * (1.0 - TAKE_PROFIT);

                tracing::info!("Entry: ${:.2}", entry_price);
                tracing::info!("Stop Loss: ${:.2} (+{:.0}%)", sl, STOP_LOSS * 100.0);
                tracing::info!("Take Profit: ${:.2} (-{:.0}%)", tp, TAKE_PROFIT * 100.0);
            } else {
                tracing::info!("--- NO TRADE ---");
            }
        } else {
            tracing::info!("[######################### SCORE #########################]\n{score}");
        }
    }

    /// Evaluates the finalgo algorithm with given arguments.
    pub async fn eval(&self, args: EvalArgs) {
        let end = utils::parse_naive_date(&args.end);

        let warmup = args.samples.saturating_add(5);
        let mut t = utils::subtract_naive_date(end, warmup);

        tracing::info!(
            "Collecting {} samples of {} tickers each...",
            args.samples,
            args.tickers.len()
        );

        let mut data = Vec::with_capacity(args.samples * args.tickers.len());

        for _ in 0..args.samples {
            let target_end = utils::add_naive_date(t, TARGET_HORIZON);

            for ticker in &args.tickers {
                data.push((t, target_end, ticker.clone()));
            }

            t = utils::add_naive_date(t, 1);
        }

        let fetched =
            utils::with_progress_async("Fetching", data.len() as u64, |span| async move {
                let database = Database::new();
                let client = Arc::new(utils::client());
                let mut fetched = Vec::with_capacity(data.len());

                for chunk in data.chunks(FETCH_CHUNK_SIZE) {
                    let mut set = JoinSet::new();

                    for (t, t_target, ticker) in chunk.iter().cloned() {
                        let mut database = database.clone();
                        let client = client.clone();
                        let span = span.clone();

                        set.spawn(async move {
                            let predict = StockData::fetch(
                                &mut database,
                                &client,
                                DataKey {
                                    end: utils::format_naive_date(t),
                                    size: CANDLE_LOOK_BACK,
                                    ticker: ticker.clone(),
                                },
                            )
                            .await;

                            let target = StockData::fetch(
                                &mut database,
                                &client,
                                DataKey {
                                    end: utils::format_naive_date(t_target),
                                    size: TARGET_HORIZON,
                                    ticker: ticker.clone(),
                                },
                            )
                            .await;

                            assert!(
                                !target.opens.is_empty(),
                                "Target dataset must contain at least 1 candle"
                            );

                            span.pb_inc(1);

                            (predict, target)
                        });
                    }

                    while let Some(res) = set.join_next().await {
                        let (predict, target) = res.expect("Fetch task failed");
                        fetched.push((predict, target));
                    }
                }

                fetched
            })
            .await;

        let mut eval = eval::build(args.stats);
        let result = eval.eval(fetched);

        tracing::info!("[######################### EVAL #########################]\n{result}");
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
    /// Output only trade-relevant information.
    #[arg(long = "trade", short = 't')]
    pub trade: bool,
    /// The target date to predict for.
    pub target: String,
    /// The ticker to use.
    pub ticker: String,
}

/// Arguments for the eval command.
#[derive(Clone, Debug, Parser)]
pub struct EvalArgs {
    /// Should the evaluator include statistics for every registered score.
    #[arg(long = "stats", short = 's')]
    pub stats: bool,
    /// The end date to use.
    pub end: String,
    /// The sample count to use.
    pub samples: usize,
    /// The ticker to use.
    pub tickers: Vec<String>,
}
