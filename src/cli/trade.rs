use crate::cli::TradeArgs;
use crate::consts::{CANDLE_LOOK_BACK, TARGET_HORIZON};
use crate::data::{DataKey, StockData};
use crate::eval::EvalRank;
use crate::score::final_score::{Decision, FinalScore};
use crate::{engine, math, utils};
use std::fmt::{Display, Formatter};

/// The minimum alpha score required for a ticker to be considered for trading.
const MIN_ALPHA_SCORE: f64 = 50.0;

/// Trade with the interface.
pub async fn trade(args: TradeArgs) {
    let target_end = utils::add_naive_date(utils::parse_naive_date(&args.target), TARGET_HORIZON);
    let target_end = utils::format_naive_date(target_end);

    let path = if args.data.as_str() == "auto" {
        find_latest_data()
    } else {
        args.data
    };

    tracing::info!("Using data file at '{path}'...");

    let data: Vec<EvalRank> =
        serde_json::from_slice(&std::fs::read(&path).expect("Failed to read data file"))
            .expect("Failed to parse data file");

    let tickers = data
        .into_iter()
        .filter(|f| f.alpha_score >= MIN_ALPHA_SCORE)
        .collect::<Vec<_>>();

    let mut trades = Vec::with_capacity(tickers.len());

    for rank in tickers {
        tracing::info!("Computing trade for '{}'...", rank.ticker);

        let data = StockData::fetch(
            &utils::client(),
            &DataKey {
                end: args.target.clone(),
                size: CANDLE_LOOK_BACK,
                ticker: rank.ticker.clone(),
            },
        )
        .await;

        let mut engine = engine::build();
        engine.compute(true, &data);

        let score = engine.score::<FinalScore>();
        let entry_price = data.closes.last().copied().unwrap_or(0.0);

        let exits = engine.indicator::<crate::indicator::exits::DynamicExits>();
        let last_idx = data.closes.len() - 1;

        let (decision, stop_loss, take_profit) =
            if rank.longs_enabled && score.decision == Decision::Long {
                let sl_dist = exits.sl_distance[last_idx];
                let tp_dist = exits.tp_distance[last_idx];

                let sl = entry_price - sl_dist;
                let tp = entry_price + tp_dist;

                (Decision::Long, sl, tp)
            } else if rank.shorts_enabled && score.decision == Decision::Short {
                let sl_dist = exits.sl_distance[last_idx];
                let tp_dist = exits.tp_distance[last_idx];

                let sl = entry_price + sl_dist;
                let tp = entry_price - tp_dist;

                (Decision::Short, sl, tp)
            } else {
                (Decision::Neutral, 0.0, 0.0)
            };

        let mut specifics = Vec::with_capacity(2);

        if !rank.longs_enabled {
            specifics.push("no longs".to_string());
        }

        if !rank.shorts_enabled {
            specifics.push("no shorts".to_string());
        }

        trades.push(Trade {
            ticker: rank.ticker,
            decision,
            target_end: target_end.clone(),
            entry_price,
            stop_loss,
            take_profit,
            alpha_score: rank.alpha_score,
            specifics,
        });
    }

    tracing::info!("[######################### TRADES #########################]");

    for trade in trades {
        println!("{trade}");
    }
}

struct Trade {
    ticker: String,
    decision: Decision,
    target_end: String,
    entry_price: f64,
    stop_loss: f64,
    take_profit: f64,
    alpha_score: f64,
    specifics: Vec<String>,
}

impl Display for Trade {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "   Ticker: '{}'", self.ticker)?;
        writeln!(f, "      Decision: {}", self.decision)?;
        writeln!(f, "      Target End: {}", self.target_end)?;
        writeln!(
            f,
            "      Entry Price: {}",
            math::round_to(self.entry_price, 2)
        )?;
        writeln!(f, "      Stop Loss: {}", math::round_to(self.stop_loss, 2))?;
        writeln!(
            f,
            "      Take Profit: {}",
            math::round_to(self.take_profit, 2)
        )?;
        writeln!(
            f,
            "      Alpha Score: {}",
            math::round_to(self.alpha_score, 2)
        )?;

        let specifics = self.specifics.join(", ");
        writeln!(f, "      Specifics: [ {specifics} ]")?;

        Ok(())
    }
}

fn find_latest_data() -> String {
    let latest = std::fs::read_dir("eval")
        .expect("Failed to read eval directory")
        .map(|entry| entry.expect("Failed to read dir entry"))
        .filter_map(|entry| {
            if entry
                .file_type()
                .expect("Failed to get file type")
                .is_file()
            {
                let file = entry.file_name();
                let file = file.to_str().expect("Failed to get file name");

                if file.ends_with(".json") {
                    let date = file.trim().strip_suffix(".json").unwrap();

                    Some(utils::parse_naive_date(date))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .max()
        .expect("No valid data found");

    let date = utils::format_naive_date(latest);

    format!("eval/{date}.json")
}
