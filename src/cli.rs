use crate::consts::{CANDLE_LOOK_BACK, FETCH_CHUNK_SIZE, TARGET_CANDLE_LOOK_BACK};
use crate::data::{DataKey, StockData};
use crate::database::Database;
use crate::{engine, eval, utils};
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
        let mut database = Database::new();

        let data = StockData::fetch(
            &mut database,
            DataKey {
                end: args.target,
                size: CANDLE_LOOK_BACK,
                ticker: args.ticker,
            },
        )
        .await;

        let mut engine = engine::build();

        let score = engine.compute(true, &data);

        tracing::info!("[######################### SCORE #########################]\n{score}");
    }

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
            let target_end = utils::add_naive_date(t, 1);

            for ticker in &args.tickers {
                data.push((t, target_end, ticker.clone()));
            }

            t = utils::add_naive_date(t, 1);
        }

        let fetched =
            utils::with_progress_async("Fetching", data.len() as u64, |span| async move {
                let database = Database::new();
                let mut fetched = Vec::with_capacity(data.len());

                for chunk in data.chunks(FETCH_CHUNK_SIZE) {
                    let mut set = JoinSet::new();

                    for (t, t_target, ticker) in chunk.iter().cloned() {
                        let mut database = database.clone();
                        let span = span.clone();

                        set.spawn(async move {
                            let predict = StockData::fetch(
                                &mut database,
                                DataKey {
                                    end: utils::format_naive_date(t),
                                    size: CANDLE_LOOK_BACK,
                                    ticker: ticker.clone(),
                                },
                            )
                            .await;

                            let target_raw = StockData::fetch(
                                &mut database,
                                DataKey {
                                    end: utils::format_naive_date(t_target),
                                    size: TARGET_CANDLE_LOOK_BACK,
                                    ticker: ticker.clone(),
                                },
                            )
                            .await;

                            assert!(
                                !target_raw.opens.is_empty(),
                                "Target dataset must contain at least 1 candle"
                            );

                            let i = target_raw.opens.len() - 1;

                            let target = StockData {
                                highs: vec![target_raw.highs[i]],
                                lows: vec![target_raw.lows[i]],
                                opens: vec![target_raw.opens[i]],
                                closes: vec![target_raw.closes[i]],
                                volumes: vec![target_raw.volumes[i]],
                            };

                            assert_eq!(target.volumes.len(), 1, "Target vols must have length 1");
                            assert_eq!(target.closes.len(), 1, "Target closes must have length 1");
                            assert_eq!(target.opens.len(), 1, "Target opens must have length 1");
                            assert_eq!(target.highs.len(), 1, "Target highs must have length 1");
                            assert_eq!(target.lows.len(), 1, "Target lows must have length 1");

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
