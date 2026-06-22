use crate::cli::RunArgs;
use crate::consts::{CANDLE_LOOK_BACK, TARGET_HORIZON};
use crate::data::{DataKey, StockData};
use crate::score::final_score::{Decision, FinalScore};
use crate::{engine, math, utils};

/// Runs the finalgo algorithm with given arguments.
pub async fn run(args: RunArgs) {
    // Calculate target end date TARGET + HORIZON
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
