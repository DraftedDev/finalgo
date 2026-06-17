use crate::consts::{CANDLE_LOOK_BACK, FETCH_CHUNK_SIZE, TARGET_HORIZON};
use crate::data::{DataCache, DataKey, StockData};
use crate::math;
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
        engine.compute(true, &data);

        let score = engine.score::<FinalScore>();
        let entry_price = data.closes.last().copied().unwrap_or(0.0);

        let exits = engine.indicator::<crate::indicator::exits::DynamicExits>();
        let last_idx = data.closes.len() - 1;

        tracing::info!("[######################### TRADE #########################]");
        tracing::info!("Ticker: {}", args.ticker);
        tracing::info!("Confidence: {}", math::round_to(score.confidence, 2));
        tracing::info!("Score: {}", math::round_to(score.score, 2));
        tracing::info!("Decision: {}", score.decision);
        tracing::info!("Predicted Target Date: {}", target_end_str);

        if score.decision.as_str() == "LONG" {
            let sl = exits.stop_loss_long[last_idx];
            let tp = exits.take_profit_long[last_idx];

            let sl_pct = (entry_price - sl) / entry_price * 100.0;
            let tp_pct = (tp - entry_price) / entry_price * 100.0;

            tracing::info!("Entry: ${:.2}", entry_price);
            tracing::info!("Stop Loss: ${:.2} (-{:.2}%)", sl, sl_pct);
            tracing::info!("Take Profit: ${:.2} (+{:.2}%)", tp, tp_pct);
        } else if score.decision.as_str() == "SHORT" {
            let sl = exits.stop_loss_short[last_idx];
            let tp = exits.take_profit_short[last_idx];

            let sl_pct = (sl - entry_price) / entry_price * 100.0;
            let tp_pct = (entry_price - tp) / entry_price * 100.0;

            tracing::info!("Entry: ${:.2}", entry_price);
            tracing::info!("Stop Loss: ${:.2} (+{:.2}%)", sl, sl_pct);
            tracing::info!("Take Profit: ${:.2} (-{:.2}%)", tp, tp_pct);
        } else {
            tracing::info!("--- NO TRADE ---");
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

        let first_t = t;
        let mut last_target_end = t;

        for _ in 0..args.samples {
            let target_end = utils::add_naive_date(t, TARGET_HORIZON);
            last_target_end = target_end;

            for ticker in &args.tickers {
                data.push((t, target_end, ticker.clone()));
            }

            t = utils::add_naive_date(t, 1);
        }

        let absolute_start = utils::subtract_naive_date(first_t, CANDLE_LOOK_BACK);
        let absolute_end = last_target_end;

        let mut cache = DataCache::new();

        let client = Arc::new(utils::client());

        tracing::info!("Pre-fetching data into cache...");
        for ticker in &args.tickers {
            cache
                .fetch_range(
                    &client,
                    ticker.clone(),
                    utils::format_naive_date(absolute_start),
                    utils::format_naive_date(absolute_end),
                )
                .await;
        }

        let cache = Arc::new(cache);

        let fetched =
            utils::with_progress_async("Fetching", data.len() as u64, |span| async move {
                let mut fetched = Vec::with_capacity(data.len());

                for chunk in data.chunks(FETCH_CHUNK_SIZE) {
                    let mut set = JoinSet::new();

                    for (t, t_target, ticker) in chunk.iter().cloned() {
                        let client = client.clone();
                        let cache = cache.clone();
                        let span = span.clone();

                        set.spawn(async move {
                            let predict = StockData::fetch(
                                &cache,
                                &client,
                                DataKey {
                                    end: utils::format_naive_date(t),
                                    size: CANDLE_LOOK_BACK,
                                    ticker: ticker.clone(),
                                },
                            )
                            .await;

                            let target = StockData::fetch(
                                &cache,
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
