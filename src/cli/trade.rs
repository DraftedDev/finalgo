use crate::cli::{Cli, TradeArgs};
use crate::consts::{CANDLE_LOOK_BACK, TARGET_HORIZON};
use crate::data::{DataKey, StockData};
use crate::eval::EvalRank;
use crate::score::final_score::{Decision, FinalScore};
use crate::{engine, math, utils};
use ibapi::accounts::AccountSummaryTags;
use ibapi::orders::{Action, OcaType, Order, TimeInForce};
use std::fmt::{Display, Formatter};
use std::time::Duration;

/// The minimum alpha score required for a symbol to be considered for trading.
const MIN_ALPHA_SCORE: f64 = 5.0;

/// The percentage of total account equity risked per trade (1%).
const RISK_PER_TRADE: f64 = 0.01;

/// The maximum percentage of total equity allowed in a single position (20%).
const MAX_ALLOCATION_PER_TRADE: f64 = 0.20;

/// The maximum total risk across all open positions.
const MAX_PORTFOLIO_HEAT: f64 = 0.15;

/// Minimum USD risk required to take a trade (prevents trading if account is too small).
const MIN_RISK_USD: f64 = 10.0;

/// Trade with the interface.
pub async fn trade(cli: Cli, args: TradeArgs) {
    let target_base_date = utils::parse_naive_date(&args.target);
    let target_end_date = utils::add_naive_date(target_base_date, TARGET_HORIZON);
    let target_end = utils::format_naive_date(target_end_date);

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
        let last_idx = data.closes.len().saturating_sub(1);

        let (decision, stop_loss, take_profit) =
            if rank.longs_enabled && score.decision == Decision::Long {
                let sl_dist = exits.sl_distance[last_idx];
                let tp_dist = exits.tp_distance[last_idx];
                (Decision::Long, entry_price - sl_dist, entry_price + tp_dist)
            } else if rank.shorts_enabled && score.decision == Decision::Short {
                let sl_dist = exits.sl_distance[last_idx];
                let tp_dist = exits.tp_distance[last_idx];
                (
                    Decision::Short,
                    entry_price + sl_dist,
                    entry_price - tp_dist,
                )
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
        let account_equity = fetch_account_equity(&client).await;
        if account_equity <= 0.0 {
            tracing::error!(
                "Failed to determine account equity. Please check your API connection. Aborting."
            );
            return;
        }
        tracing::info!("Current Account Equity: ${:.2}", account_equity);

        let open_positions = fetch_open_position_count(&client).await;
        let mut current_heat = open_positions as f64 * RISK_PER_TRADE;
        tracing::info!(
            "Current Open Positions: {} | Current Portfolio Heat: {:.1}%",
            open_positions,
            current_heat * 100.0
        );

        let mut current_id = client.next_valid_order_id().await.unwrap();

        for trade in trades {
            if trade.decision == Decision::Neutral {
                tracing::info!("Skipping NEUTRAL decision for '{}'.", trade.symbol);
                continue;
            }

            if current_heat + RISK_PER_TRADE > MAX_PORTFOLIO_HEAT {
                tracing::warn!(
                    "Skipping '{}'. Portfolio Heat limit reached ({:.1}% + {:.1}% > {:.1}%).",
                    trade.symbol,
                    current_heat * 100.0,
                    RISK_PER_TRADE * 100.0,
                    MAX_PORTFOLIO_HEAT * 100.0
                );
                continue;
            }

            tracing::info!("Trading: {trade:#?}");

            let quantity =
                match calculate_quantity(account_equity, trade.entry_price, trade.stop_loss) {
                    Some(q) => q,
                    None => {
                        tracing::info!(
                            "Skipping '{}' due to position sizing constraints.",
                            trade.symbol
                        );
                        continue;
                    }
                };

            let risk_usd = (trade.entry_price - trade.stop_loss).abs() * quantity;
            tracing::info!(
                "Automated Sizing for {}: {} shares (Risk: ${:.2}, Cost: ${:.2})",
                trade.symbol,
                quantity,
                risk_usd,
                quantity * trade.entry_price
            );

            tracing::info!("Executing Order for '{}'...", trade.symbol);
            let contract = utils::contract(&trade.symbol);

            let parent_id = current_id;
            current_id += 1;
            let tp_id = current_id;
            current_id += 1;
            let sl_id = current_id;
            current_id += 1;
            let time_exit_id = current_id;
            current_id += 1;

            let (entry_action, exit_action) = if trade.decision == Decision::Long {
                (Action::Buy, Action::Sell)
            } else {
                (Action::Sell, Action::Buy)
            };

            let gat_string = format!("{} 15:50:00 US/Eastern", target_end_date.format("%Y%m%d"));
            let oca_group = format!("OCA_{}_{}", trade.symbol, parent_id);

            let parent = Order {
                order_id: parent_id,
                action: entry_action,
                total_quantity: quantity,
                order_type: "MKT".to_string(),
                tif: TimeInForce::GoodTilCanceled,
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
                tif: TimeInForce::GoodTilCanceled,
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
                tif: TimeInForce::GoodTilCanceled,
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

            current_heat += RISK_PER_TRADE;
        }

        tracing::info!(
            "Orders successfully submitted to IBKR. Keeping connection alive for 5 seconds to allow processing..."
        );
        tokio::time::sleep(Duration::from_secs(5)).await;

        tracing::info!("Execution complete.");
    } else {
        tracing::info!("Aborting trades...");
    }
}

async fn fetch_account_equity(client: &ibapi::Client) -> f64 {
    let group = "All".into();

    let mut sub = match client
        .account_summary(&group, &[AccountSummaryTags::NET_LIQUIDATION])
        .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to subscribe to account summary: {:?}", e);
            return 0.0;
        }
    };

    let results = sub.collect_for(Duration::from_secs(2)).await;

    sub.cancel().await;

    for result in results {
        if let ibapi::accounts::AccountSummaryResult::Summary(summary) = result
            && summary.tag == AccountSummaryTags::NET_LIQUIDATION
            && (summary.currency == "USD"
                || summary.currency == "BASE"
                || summary.currency == "EUR")
            && let Ok(val) = summary.value.parse::<f64>()
        {
            return val;
        }
    }

    tracing::warn!("Could not find NetLiquidation in account summary.");
    0.0
}

async fn fetch_open_position_count(client: &ibapi::Client) -> usize {
    let mut sub = match client.positions().await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to subscribe to positions: {:?}", e);
            return 0;
        }
    };

    let results = sub.collect_for(Duration::from_secs(2)).await;
    sub.cancel().await;

    results.len()
}

fn calculate_quantity(account_equity: f64, entry_price: f64, stop_loss_price: f64) -> Option<f64> {
    let risk_amount = account_equity * RISK_PER_TRADE;

    if risk_amount < MIN_RISK_USD {
        tracing::warn!("Account equity too low to safely risk ${MIN_RISK_USD}. Skipping.");
        return None;
    }

    let stop_distance = (entry_price - stop_loss_price).abs();

    if stop_distance < 1e-9 {
        tracing::warn!("Stop distance is zero. Skipping.");
        return None;
    }

    let raw_shares = risk_amount / stop_distance;
    let position_cost = raw_shares * entry_price;
    let max_allowed_cost = account_equity * MAX_ALLOCATION_PER_TRADE;

    let final_shares = if position_cost > max_allowed_cost {
        let capped_shares = max_allowed_cost / entry_price;
        capped_shares.floor()
    } else {
        raw_shares.floor()
    };

    if final_shares < 1.0 {
        tracing::warn!(
            "Calculated quantity is < 1 share. Stop loss is too wide for current equity."
        );
        return None;
    }

    Some(final_shares)
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

                file.strip_suffix(".json").map(utils::parse_naive_date)
            } else {
                None
            }
        })
        .max()
        .expect("No valid data found");

    let date = utils::format_naive_date(latest);
    format!("eval/{date}.json")
}
