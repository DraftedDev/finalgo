use crate::consts::{CANDLE_LOOK_BACK, FETCH_CHUNK_SIZE, TARGET_HORIZON};
use crate::data::{DataCache, DataKey, StockData};
use crate::math;
use crate::score::final_score::{Decision, FinalScore};
use crate::utils::FastMap;
use crate::{engine, eval, utils};
use clap::Parser;
use std::sync::Arc;
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

        let data = StockData::fetch(
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

        if score.decision == Decision::Long {
            let sl_dist = exits.sl_distance[last_idx];
            let tp_dist = exits.tp_distance[last_idx];

            let sl = entry_price - sl_dist;
            let tp = entry_price + tp_dist;

            let sl_pct = (sl_dist / entry_price) * 100.0;
            let tp_pct = (tp_dist / entry_price) * 100.0;

            tracing::info!("Entry: ${:.2}", entry_price);
            tracing::info!("Stop Loss: ${:.2} (-{:.2}%)", sl, sl_pct);
            tracing::info!("Take Profit: ${:.2} (+{:.2}%)", tp, tp_pct);
        } else if score.decision == Decision::Short {
            let sl_dist = exits.sl_distance[last_idx];
            let tp_dist = exits.tp_distance[last_idx];

            let sl = entry_price + sl_dist;
            let tp = entry_price - tp_dist;

            let sl_pct = (sl_dist / entry_price) * 100.0;
            let tp_pct = (tp_dist / entry_price) * 100.0;

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

        let mut grouped_data: FastMap<String, Vec<(StockData, StockData)>> = FastMap::default();

        for ticker in &args.tickers {
            grouped_data.insert(ticker.clone(), Vec::with_capacity(args.samples));
        }

        let tickers = args.tickers.clone();

        let fetched = utils::with_progress("Collecting", data.len() as u64, |span| {
            for chunk in data.chunks(FETCH_CHUNK_SIZE) {
                for (t, t_target, ticker) in chunk.iter().cloned() {
                    let predict = cache
                        .get_stock_data(&DataKey {
                            end: utils::format_naive_date(t),
                            size: CANDLE_LOOK_BACK,
                            ticker: ticker.clone(),
                        })
                        .expect("Invalid cache state");

                    let target = cache
                        .get_stock_data(&DataKey {
                            end: utils::format_naive_date(t_target),
                            size: TARGET_HORIZON,
                            ticker: ticker.clone(),
                        })
                        .expect("Invalid cache state");

                    assert!(
                        !target.opens.is_empty(),
                        "Target dataset must contain at least 1 candle"
                    );

                    span.pb_inc(1);

                    grouped_data
                        .get_mut(&ticker)
                        .unwrap()
                        .push((predict, target));
                }
            }

            tickers
                .iter()
                .map(|t| (t.clone(), grouped_data.remove(t).unwrap()))
                .collect::<Vec<(String, Vec<(StockData, StockData)>)>>()
        });

        let eval = eval::build(args.stats);

        if args.rank {
            let result = eval.rank(fetched);

            if let Some(path) = args.out {
                tracing::info!("Writing output to '{path}'...");
                let out =
                    serde_json::to_string_pretty(&result).expect("Failed to serialize output");

                std::fs::write(path, out).expect("Failed to write output");
            } else {
                let out = result
                    .into_iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");

                tracing::info!("[######################### RANK #########################]\n{out}");
            }
        } else {
            let data = fetched
                .into_iter()
                .flat_map(|(_, data)| data)
                .collect::<Vec<_>>();

            let result = eval.eval(data);

            if let Some(path) = args.out {
                tracing::info!("Writing output to '{path}'...");
                let out =
                    serde_json::to_string_pretty(&result).expect("Failed to serialize output");

                std::fs::write(path, out).expect("Failed to write output");
            } else {
                tracing::info!(
                    "[######################### EVAL #########################]\n{result}"
                );
            }
        }
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
    /// The sample count to use.
    #[arg(long = "samples", short = 'c', default_value_t = 250)]
    pub samples: usize,
    /// Should the evaluator rank the tickers.
    #[arg(long = "rank", short = 'r')]
    pub rank: bool,
    /// If set, the JSON output will be written to the given path.
    #[arg(long = "out", short = 'o')]
    pub out: Option<String>,
    /// The end date to use.
    pub end: String,
    /// The ticker to use.
    pub tickers: Vec<String>,
}
