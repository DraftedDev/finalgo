use crate::eval::metric::{Metric, MetricInput};
use crate::indicator::exits::DynamicExits;
use crate::score::final_score::{Decision, FinalScore};
use crate::utils::{Value, ValueMap};

/// 50 basis points round-trip (25 bps entry + 25 bps exit).
const FRICTION: f64 = 0.005;

/// Scaling factor to make the Alpha Score human-readable.
const ALPHA_SCALE: f64 = 30_000.0;

/// # Profit-Loss Metric
///
/// Computes trading stats to eval the performance of the algorithm in terms of profit/loss.
///
/// Uses the [DynamicExits] indicator for ATR-based Take-Profit and Stop-Loss distances.
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

        let mut count = 0;
        let mut mean = 0.0;
        let mut m2 = 0.0;

        for sample in result {
            let decision = sample.engine.score::<FinalScore>().decision;

            if decision == Decision::Neutral {
                continue;
            }

            let target = &sample.target;
            if target.opens.is_empty() {
                continue;
            }

            let entry = target.opens[0];
            if !entry.is_finite() || entry.abs() < 1e-12 {
                continue;
            }

            let exits = sample.engine.indicator::<DynamicExits>();
            if exits.sl_distance.is_empty() {
                continue;
            }
            let last_idx = exits.sl_distance.len() - 1;

            let sl_dist = exits.sl_distance[last_idx];
            let tp_dist = exits.tp_distance[last_idx];

            let mut trade_pnl = 0.0;
            let mut exited = false;

            for day in 0..target.opens.len() {
                let day_open = target.opens[day];
                let day_high = target.highs[day];
                let day_low = target.lows[day];
                let day_close = target.closes[day];

                match decision {
                    Decision::Long => {
                        let actual_sl = entry - sl_dist;
                        let actual_tp = entry + tp_dist;

                        if day_open <= actual_sl || day_open >= actual_tp {
                            trade_pnl = (day_open - entry) / entry - FRICTION;
                            exited = true;
                            break;
                        }

                        if day_low <= actual_sl {
                            trade_pnl = (actual_sl - entry) / entry - FRICTION;
                            exited = true;
                            break;
                        } else if day_high >= actual_tp {
                            trade_pnl = (actual_tp - entry) / entry - FRICTION;
                            exited = true;
                            break;
                        }

                        if day == target.opens.len() - 1 {
                            trade_pnl = (day_close - entry) / entry - FRICTION;
                            exited = true;
                        }
                    }
                    Decision::Short => {
                        let actual_sl = entry + sl_dist;
                        let actual_tp = entry - tp_dist;

                        if day_open >= actual_sl || day_open <= actual_tp {
                            trade_pnl = (entry - day_open) / entry - FRICTION;
                            exited = true;
                            break;
                        }

                        if day_high >= actual_sl {
                            trade_pnl = (entry - actual_sl) / entry - FRICTION;
                            exited = true;
                            break;
                        } else if day_low <= actual_tp {
                            trade_pnl = (entry - actual_tp) / entry - FRICTION;
                            exited = true;
                            break;
                        }

                        if day == target.opens.len() - 1 {
                            trade_pnl = (entry - day_close) / entry - FRICTION;
                            exited = true;
                        }
                    }
                    Decision::Neutral => unreachable!(),
                }
            }

            if !exited {
                continue;
            }

            trades_taken += 1;
            total_return += trade_pnl;

            if trade_pnl > 0.0 {
                wins += 1;
                gross_profit += trade_pnl;
            } else {
                losses += 1;
                gross_loss += trade_pnl.abs();
            }

            count += 1;
            let delta = trade_pnl - mean;
            mean += delta / count as f64;
            let delta2 = trade_pnl - mean;
            m2 += delta * delta2;
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

        let variance = if count > 1 {
            m2 / (count - 1) as f64
        } else {
            0.0
        };
        let std_dev = variance.sqrt();
        let sharpe = if std_dev > 1e-9 {
            (mean / std_dev) * (52.0_f64.sqrt())
        } else {
            0.0
        };

        let total_samples = result.len();
        let trade_frequency = if total_samples > 0 {
            trades_taken as f64 / total_samples as f64
        } else {
            0.0
        };

        let mut alpha_score = 0.0;

        let is_valid = trade_frequency >= 0.15
            && profit_factor >= 1.10
            && avg_win > FRICTION
            && win_rate > 0.3333
            && sharpe > 0.0;

        if is_valid {
            let capped_pf = profit_factor.min(5.0);
            let raw_alpha = expectancy * (capped_pf - 1.0) * trade_frequency * sharpe;
            alpha_score = raw_alpha * ALPHA_SCALE;
        }

        ValueMap::new()
            .with("pnl_trades_taken", Value::Int(trades_taken as i64))
            .with("pnl_win_rate", Value::Percent(win_rate))
            .with("pnl_total_return", Value::Percent(total_return))
            .with("pnl_avg_win", Value::Percent(avg_win))
            .with("pnl_avg_loss", Value::Percent(avg_loss))
            .with("pnl_profit_factor", Value::Float(profit_factor))
            .with("pnl_expectancy", Value::Percent(expectancy))
            .with("pnl_sharpe", Value::Float(sharpe))
            .with("pnl_alpha_score", Value::Float(alpha_score))
    }
}
