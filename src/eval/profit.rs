use crate::eval::metric::{Metric, MetricInput};
use crate::score::final_score::FinalScore;
use crate::utils::{Value, ValueMap};
use std::f64;

/// 20 basis points round-trip (10 bps entry + 10bps exit).
const FRICTION: f64 = 0.002;

/// 3% Take Profit.
pub const TAKE_PROFIT: f64 = 0.03;

/// 2% Stop Loss.
pub const STOP_LOSS: f64 = 0.02;

/// # Profit-Loss Metric
///
/// Computes the average profit/loss per trade and a few more statistics
/// to evaluate the performance of the algorithm in terms of profit/loss.
pub struct ProfitLossMetric;

impl ProfitLossMetric {
    /// The key for the trades taken metric value.
    pub const TRADES_TAKEN_KEY: &'static str = "pnl_trades_taken";

    /// The key for the win rate (%) metric value.
    pub const WIN_RATE_KEY: &'static str = "pnl_win_rate";

    /// The key for the total return (%) metric value.
    pub const TOTAL_RETURN_KEY: &'static str = "pnl_total_return";

    /// The key for the average win (%) metric value.
    pub const AVG_WIN_KEY: &'static str = "pnl_avg_win";

    /// The key for the average loss (%) metric value.
    pub const AVG_LOSS_KEY: &'static str = "pnl_avg_loss";

    /// The key for the profit factor metric value.
    pub const PROFIT_FACTOR_KEY: &'static str = "pnl_profit_factor";

    /// The key for the expectancy (%) metric value.
    pub const EXPECTANCY_KEY: &'static str = "pnl_expectancy";

    /// The key for the Sharpe ratio metric value.
    pub const SHARPE_KEY: &'static str = "pnl_sharpe";
}

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
            let decision_str = sample
                .score
                .get(FinalScore::FINAL_SCORE_DECISION_KEY)
                .as_str();
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

            let mut trade_pnl = 0.0;

            for day in 0..target.opens.len() {
                let day_high = target.highs[day];
                let day_low = target.lows[day];
                let day_close = target.closes[day];

                match decision {
                    Decision::Long => {
                        let high_return = (day_high - entry) / entry;
                        let low_return = (day_low - entry) / entry;

                        if low_return <= -STOP_LOSS {
                            trade_pnl = -STOP_LOSS - FRICTION;
                            break;
                        } else if high_return >= TAKE_PROFIT {
                            trade_pnl = TAKE_PROFIT - FRICTION;
                            break;
                        }

                        if day == target.opens.len() - 1 {
                            let close_return = (day_close - entry) / entry;
                            trade_pnl = close_return - FRICTION;
                        }
                    }
                    Decision::Short => {
                        let high_return = (day_high - entry) / entry;
                        let low_return = (day_low - entry) / entry;

                        if high_return >= STOP_LOSS {
                            trade_pnl = -STOP_LOSS - FRICTION;
                            break;
                        } else if -low_return >= TAKE_PROFIT {
                            trade_pnl = TAKE_PROFIT - FRICTION;
                            break;
                        }

                        if day == target.opens.len() - 1 {
                            let close_return = (entry - day_close) / entry;
                            trade_pnl = close_return - FRICTION;
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
            .with(Self::TRADES_TAKEN_KEY, Value::Int(trades_taken as i64))
            .with(Self::WIN_RATE_KEY, Value::Percent(win_rate))
            .with(Self::TOTAL_RETURN_KEY, Value::Percent(total_return))
            .with(Self::AVG_WIN_KEY, Value::Percent(avg_win))
            .with(Self::AVG_LOSS_KEY, Value::Percent(avg_loss))
            .with(Self::PROFIT_FACTOR_KEY, Value::Float(profit_factor))
            .with(Self::EXPECTANCY_KEY, Value::Percent(expectancy))
            .with(Self::SHARPE_KEY, Value::Float(sharpe))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Decision {
    Long,
    Short,
    Neutral,
}
