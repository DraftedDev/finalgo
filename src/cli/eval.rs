use crate::cli::{Cli, EvalArgs};
use crate::consts::{CANDLE_LOOK_BACK, FETCH_CHUNK_SIZE, TARGET_HORIZON};
use crate::data::{DataCache, DataKey, StockData};
use crate::utils;
use crate::utils::FastMap;
use std::sync::Arc;
use tracing_indicatif::span_ext::IndicatifSpanExt;

/// Evaluates the finalgo algorithm with given arguments.
pub async fn eval(cli: Cli, mut args: EvalArgs) {
    let end = utils::parse_naive_date(&args.end);

    let shift = args
        .samples
        .saturating_add(TARGET_HORIZON)
        .saturating_sub(1);
    let mut t = utils::subtract_naive_date(end, shift);

    tracing::info!(
        "Collecting {} samples of {} symbols each...",
        args.samples,
        args.symbols.len()
    );

    let mut data = Vec::with_capacity(args.samples * args.symbols.len());

    let first_t = t;

    for _ in 0..args.samples {
        let target_end = utils::add_naive_date(t, TARGET_HORIZON);

        for symbol in &args.symbols {
            data.push((t, target_end, symbol.clone()));
        }

        t = utils::add_naive_date(t, 1);
    }

    let absolute_start = utils::subtract_naive_date(first_t, CANDLE_LOOK_BACK);

    let absolute_end = end;

    let mut cache = DataCache::new();
    let client = Arc::new(utils::client(cli.paper).await);

    tracing::info!("Pre-fetching data into cache...");
    for symbol in &args.symbols {
        cache
            .fetch_range(
                &client,
                symbol.clone(),
                utils::format_naive_date(absolute_start),
                utils::format_naive_date(absolute_end),
            )
            .await;
    }

    let cache = Arc::new(cache);

    let mut grouped_data: FastMap<String, Vec<(StockData, StockData)>> = FastMap::default();

    for symbol in &args.symbols {
        grouped_data.insert(symbol.clone(), Vec::with_capacity(args.samples));
    }

    let symbols = args.symbols.clone();

    if let Some(path) = args.out.as_mut()
        && path.as_str() == "auto"
    {
        *path = format!("eval/{}.json", args.end);
    }

    let fetched = utils::with_progress("Collecting", data.len() as u64, |span| {
        for chunk in data.chunks(FETCH_CHUNK_SIZE) {
            for (t, t_target, symbol) in chunk.iter().cloned() {
                let predict = cache
                    .get_stock_data(&DataKey {
                        end: utils::format_naive_date(t),
                        size: CANDLE_LOOK_BACK,
                        symbol: symbol.clone(),
                    })
                    .expect("Invalid cache state");

                let target = cache
                    .get_stock_data(&DataKey {
                        end: utils::format_naive_date(t_target),
                        size: TARGET_HORIZON,
                        symbol: symbol.clone(),
                    })
                    .expect("Invalid cache state");

                assert!(
                    !target.opens.is_empty(),
                    "Target dataset must contain at least 1 candle"
                );

                span.pb_inc(1);

                grouped_data
                    .get_mut(&symbol)
                    .unwrap()
                    .push((predict, target));
            }
        }

        symbols
            .iter()
            .map(|t| (t.clone(), grouped_data.remove(t).unwrap()))
            .collect::<Vec<(String, Vec<(StockData, StockData)>)>>()
    });

    let eval = crate::eval::build(args.stats, args.rank);

    if args.rank {
        let result = eval.rank(fetched);

        if let Some(path) = args.out {
            tracing::info!("Writing output to '{path}'...");
            let out = serde_json::to_string_pretty(&result).expect("Failed to serialize output");

            std::fs::write(path, out).expect("Failed to write output");
        }

        let out = result
            .into_iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        tracing::info!("[######################### RANK #########################]\n{out}");
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
