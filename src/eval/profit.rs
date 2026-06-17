use crate::eval::metric::{Metric, MetricInput};
use crate::indicator::exits::DynamicExits;
use crate::score::final_score::FinalScore;
use crate::utils::{Value, ValueMap};
use std::f64;

/// 20 basis points round-trip (10 bps entry + 10bps exit).
const FRICTION: f64 = 0.002;

/// # Profit-Loss Metric
///
/// Computes the average profit/loss per trade and a few more statistics
/// to evaluate the performance of the algorithm in terms of profit/loss.
///
/// Uses the [DynamicExits] indicator for volatility-normalized (ATR-based)
/// Take-Profit and Stop-Loss levels, ensuring multi-asset compatibility.
pub struct ProfitLossMetric;

impl Metric for ProfitLossMetric {
    fn name(&self) -> String {
        "profit_loss".to_string()
    }

    fn compute(&self, result: &[MetricInput]) -> ValueMap {
        let mut trades_taken = 0;
        let mut wins = 0;
        let mut losses = 0;

        let mut gross_profit = 0.0;
        let mut gross_loss = 0.0;
        let mut total_return = 0.0;

        let mut trade_returns: Vec<f64> = Vec::new();

        for sample in result {
            let decision_str = sample.engine.score::<FinalScore>().decision.as_str();

            let decision = match decision_str.trim().to_ascii_uppercase().as_str() {
                "LONG" => Decision::Long,
                "SHORT" => Decision::Short,
                _ => Decision::Neutral,
            };

            if decision == Decision::Neutral {
                continue;
            }

            trades_taken += 1;
            let target = &sample.target;
            let entry = target.opens[0];

            if entry.abs() < 1e-12 {
                continue;
            }

            let exits = sample.engine.indicator::<DynamicExits>();
            let last_idx = exits.stop_loss_long.len() - 1;

            let sl_long = exits.stop_loss_long[last_idx];
            let tp_long = exits.take_profit_long[last_idx];
            let sl_short = exits.stop_loss_short[last_idx];
            let tp_short = exits.take_profit_short[last_idx];

            let mut trade_pnl = 0.0;

            for day in 0..target.opens.len() {
                let day_high = target.highs[day];
                let day_low = target.lows[day];
                let day_close = target.closes[day];

                match decision {
                    Decision::Long => {
                        if day_low <= sl_long {
                            trade_pnl = (sl_long - entry) / entry - FRICTION;
                            break;
                        } else if day_high >= tp_long {
                            trade_pnl = (tp_long - entry) / entry - FRICTION;
                            break;
                        }

                        if day == target.opens.len() - 1 {
                            trade_pnl = (day_close - entry) / entry - FRICTION;
                        }
                    }
                    Decision::Short => {
                        if day_high >= sl_short {
                            trade_pnl = (entry - sl_short) / entry - FRICTION;
                            break;
                        } else if day_low <= tp_short {
                            trade_pnl = (entry - tp_short) / entry - FRICTION;
                            break;
                        }

                        if day == target.opens.len() - 1 {
                            trade_pnl = (entry - day_close) / entry - FRICTION;
                        }
                    }
                    Decision::Neutral => unreachable!(),
                }
            }

            total_return += trade_pnl;
            trade_returns.push(trade_pnl);

            if trade_pnl > 0.0 {
                wins += 1;
                gross_profit += trade_pnl;
            } else {
                losses += 1;
                gross_loss += trade_pnl.abs();
            }
        }

        let win_rate = if trades_taken > 0 {
            wins as f64 / trades_taken as f64
        } else {
            0.0
        };
        let avg_win = if wins > 0 {
            gross_profit / wins as f64
        } else {
            0.0
        };
        let avg_loss = if losses > 0 {
            -(gross_loss / losses as f64)
        } else {
            0.0
        };

        let profit_factor = if gross_loss > 1e-9 {
            gross_profit / gross_loss
        } else {
            99.99
        };
        let expectancy = if trades_taken > 0 {
            total_return / trades_taken as f64
        } else {
            0.0
        };

        let mean_return = expectancy;
        let variance = if trades_taken > 1 {
            trade_returns
                .iter()
                .map(|r| (r - mean_return).powi(2))
                .sum::<f64>()
                / (trades_taken - 1) as f64
        } else {
            0.0
        };
        let std_dev = variance.sqrt();
        let sharpe = if std_dev > 1e-9 {
            (mean_return / std_dev) * (52.0_f64.sqrt())
        } else {
            0.0
        };

        ValueMap::new()
            .with("pnl_trades_taken", Value::Int(trades_taken as i64))
            .with("pnl_win_rate", Value::Percent(win_rate))
            .with("pnl_total_return", Value::Percent(total_return))
            .with("pnl_avg_win", Value::Percent(avg_win))
            .with("pnl_avg_loss", Value::Percent(avg_loss))
            .with("pnl_profit_factor", Value::Float(profit_factor))
            .with("pnl_expectancy", Value::Percent(expectancy))
            .with("pnl_sharpe", Value::Float(sharpe))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Decision {
    Long,
    Short,
    Neutral,
}
