use crate::engine::Context;
use crate::indicator::Indicator;
use crate::indicator::atr::AvgTrueRange;
use std::any::Any;

/// How many ATR units away to place the Stop-Loss.
const LOSS_MULTI: f64 = 2.0;

/// How many ATR units away to place the Take-Profit.
const PROFIT_MULTI: f64 = 3.0;

/// # Dynamic Exits Indicator
///
/// Computes Take-Profit and Stop-Loss price levels based on the Average True Range (ATR).
pub struct DynamicExits {
    /// Stop loss price levels for LONG positions.
    pub stop_loss_long: Vec<f64>,

    /// Take profit price levels for LONG positions.
    pub take_profit_long: Vec<f64>,

    /// Stop loss price levels for SHORT positions.
    pub stop_loss_short: Vec<f64>,

    /// Take profit price levels for SHORT positions.
    pub take_profit_short: Vec<f64>,
}

impl DynamicExits {
    /// Creates a new [DynamicExits] indicator.
    pub fn new() -> Self {
        Self {
            stop_loss_long: Vec::new(),
            take_profit_long: Vec::new(),
            stop_loss_short: Vec::new(),
            take_profit_short: Vec::new(),
        }
    }
}

impl Indicator for DynamicExits {
    fn name() -> String {
        "dynamic_exits".to_string()
    }

    fn compute(&mut self, ctx: Context) {
        let data = ctx.data();
        let closes = &data.closes;
        let len = closes.len();

        let atr = ctx.indicator::<AvgTrueRange<14>>();

        self.stop_loss_long = vec![0.0; len];
        self.take_profit_long = vec![0.0; len];
        self.stop_loss_short = vec![0.0; len];
        self.take_profit_short = vec![0.0; len];

        for (i, (&close, &current_atr)) in closes.iter().zip(&atr.atr).enumerate() {
            let atr_val = if current_atr.is_finite() && current_atr > 0.0 {
                current_atr
            } else {
                close * 0.02
            };

            let sl_distance = atr_val * LOSS_MULTI;
            let tp_distance = atr_val * PROFIT_MULTI;

            self.stop_loss_long[i] = close - sl_distance;
            self.take_profit_long[i] = close + tp_distance;

            self.stop_loss_short[i] = close + sl_distance;
            self.take_profit_short[i] = close - tp_distance;
        }
    }

    fn is_computed(&self) -> bool {
        !self.stop_loss_long.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
