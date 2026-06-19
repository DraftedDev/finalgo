use crate::engine::Context;
use crate::indicator::Indicator;
use crate::indicator::atr::AvgTrueRange;
use std::any::Any;

/// How many ATR units away to place the Stop-Loss.
const LOSS_MULTI: f64 = 1.5;

/// How many ATR units away to place the Take-Profit.
const PROFIT_MULTI: f64 = 2.0;

/// # Dynamic Exits Indicator
///
/// Computes the absolute dollar *distance* for Take-Profit and Stop-Loss based on the ATR.
pub struct DynamicExits {
    /// The absolute dollar distance for the Stop Loss.
    pub sl_distance: Vec<f64>,

    /// The absolute dollar distance for the Take Profit.
    pub tp_distance: Vec<f64>,
}

impl DynamicExits {
    pub fn new() -> Self {
        Self {
            sl_distance: Vec::new(),
            tp_distance: Vec::new(),
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

        self.sl_distance.clear();
        self.tp_distance.clear();
        self.sl_distance.reserve(len);
        self.tp_distance.reserve(len);

        for (i, &close) in closes.iter().enumerate() {
            let current_atr = atr.atr.get(i).copied().unwrap_or(f64::NAN);

            let atr_val = if current_atr.is_finite() && current_atr > 0.0 {
                current_atr
            } else {
                close * 0.02
            };

            self.sl_distance.push(atr_val * LOSS_MULTI);
            self.tp_distance.push(atr_val * PROFIT_MULTI);
        }
    }

    fn is_computed(&self) -> bool {
        !self.sl_distance.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
