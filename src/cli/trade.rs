use crate::cli::{Cli, TradeArgs};
use crate::consts::{CANDLE_LOOK_BACK, TARGET_HORIZON};
use crate::data::{DataKey, StockData};
use crate::eval::EvalRank;
use crate::score::final_score::{Decision, FinalScore};
use crate::{engine, math, utils};
use chrono::Utc;
use ibapi::orders::{Action, OcaType, Order, TimeInForce};
use std::fmt::{Display, Formatter};

/// The minimum alpha score required for a symbol to be considered for trading.
const MIN_ALPHA_SCORE: f64 = 5.0;

/// Trade with the interface.
pub async fn trade(cli: Cli, args: TradeArgs) {
    // Calculate end date of prediction TARGET + HORIZON
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

    let symbols = data
        .into_iter()
        .filter(|f| f.alpha_score >= MIN_ALPHA_SCORE)
        .collect::<Vec<_>>();

    let mut trades = Vec::with_capacity(symbols.len());

    let client = utils::client(cli.paper).await;

    for rank in symbols {
        tracing::info!("Computing trade for '{}'...", rank.symbol);

        let data = StockData::fetch(
            &client,
            &DataKey {
                end: args.target.clone(),
                size: CANDLE_LOOK_BACK,
                symbol: rank.symbol.clone(),
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
            symbol: rank.symbol,
            decision,
            target_end: target_end.clone(),
            entry_price: math::round_to(entry_price, 2),
            stop_loss: math::round_to(stop_loss, 2),
            take_profit: math::round_to(take_profit, 2),
            alpha_score: math::round_to(rank.alpha_score, 2),
            specifics,
        });
    }

    tracing::info!("[######################### TRADES #########################]");

    for trade in &trades {
        println!("{trade}");
    }

    if utils::prompt_confirm("Proceed with trades?") {
        for trade in trades {
            if trade.decision == Decision::Neutral {
                tracing::info!("Skipping NEUTRAL decision for '{}'.", trade.symbol);
                continue;
            }

            tracing::info!("Trading: {trade:#?}");
            let quantity = utils::prompt_float("How much to trade?");

            if quantity <= 0.0 {
                tracing::info!("Quantity <= 0.0. Skipping trade...");
                continue;
            }

            tracing::info!("Executing Order for '{}'...", trade.symbol);

            let contract = utils::contract(&trade.symbol);

            let parent_id = client.next_valid_order_id().await.unwrap();
            let tp_id = parent_id + 1;
            let sl_id = parent_id + 2;
            let time_exit_id = parent_id + 3;

            let (entry_action, exit_action) = if trade.decision == Decision::Long {
                (Action::Buy, Action::Sell)
            } else {
                (Action::Sell, Action::Buy)
            };

            let today = Utc::now().date_naive();
            let target_date = utils::add_naive_date(today, TARGET_HORIZON);
            let gat_string = format!("{} 15:50:00 US/Eastern", target_date.format("%Y%m%d"));

            let oca_group = format!("OCA_{}", parent_id);

            let parent = Order {
                order_id: parent_id,
                action: entry_action,
                total_quantity: quantity,
                order_type: "MKT".to_string(),
                transmit: false,
                ..Default::default()
            };

            let take_profit = Order {
                order_id: tp_id,
                action: exit_action,
                total_quantity: quantity,
                order_type: "LMT".to_string(),
                limit_price: Some(trade.take_profit),
                parent_id,
                oca_group: oca_group.clone(),
                oca_type: OcaType::CancelWithBlock,
                transmit: false,
                ..Default::default()
            };

            let stop_loss = Order {
                order_id: sl_id,
                action: exit_action,
                total_quantity: quantity,
                order_type: "STP".to_string(),
                aux_price: Some(trade.stop_loss),
                parent_id,
                oca_group: oca_group.clone(),
                oca_type: OcaType::CancelWithBlock,
                transmit: false,
                ..Default::default()
            };

            let time_exit = Order {
                order_id: time_exit_id,
                action: exit_action,
                total_quantity: quantity,
                order_type: "MKT".to_string(),
                parent_id,
                oca_group: oca_group.clone(),
                oca_type: OcaType::CancelWithBlock,
                tif: TimeInForce::GoodTilCanceled,
                good_after_time: gat_string,
                transmit: true,
                ..Default::default()
            };

            client
                .submit_order(parent_id, &contract, &parent)
                .await
                .expect("Failed to place parent");
            client
                .submit_order(tp_id, &contract, &take_profit)
                .await
                .expect("Failed to place TP");
            client
                .submit_order(sl_id, &contract, &stop_loss)
                .await
                .expect("Failed to place SL");
            client
                .submit_order(time_exit_id, &contract, &time_exit)
                .await
                .expect("Failed to place Time Exit");

            tracing::info!("Orders for '{}' placed successfully.", trade.symbol);
        }

        tracing::info!(
            "Orders successfully submitted to IBKR. Keeping connection alive for 5 seconds to allow processing..."
        );
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        tracing::info!("Execution complete.");
    } else {
        tracing::info!("Aborting trades...");
    }
}

#[derive(Debug)]
struct Trade {
    symbol: String,
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
        writeln!(f, "   Symbol: '{}'", self.symbol)?;
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
