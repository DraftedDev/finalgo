use crate::cli::EvalArgs;
use crate::consts::{CANDLE_LOOK_BACK, FETCH_CHUNK_SIZE, TARGET_HORIZON};
use crate::data::{DataCache, DataKey, StockData};
use crate::utils;
use crate::utils::FastMap;
use std::sync::Arc;
use tracing_indicatif::span_ext::IndicatifSpanExt;

/// Evaluates the finalgo algorithm with given arguments.
pub async fn eval(mut args: EvalArgs) {
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

    if let Some(path) = args.out.as_mut()
        && path.as_str() == "auto"
    {
        *path = format!("eval/{}.json", args.end);
    }

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

    let eval = crate::eval::build(args.stats);

    if args.rank {
        let result = eval.rank(fetched);

        if let Some(path) = args.out {
            tracing::info!("Writing output to '{path}'...");
            let out = serde_json::to_string_pretty(&result).expect("Failed to serialize output");

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
            let out = serde_json::to_string_pretty(&result).expect("Failed to serialize output");

            std::fs::write(path, out).expect("Failed to write output");
        } else {
            tracing::info!("[######################### EVAL #########################]\n{result}");
        }
    }
}
