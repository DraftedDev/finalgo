use crate::engine::Context;
use crate::indicator::Indicator;
use crate::indicator::atr::AvgTrueRange;
use crate::indicator::er::EfficiencyRatio;
use crate::indicator::regime::MarketRegime;
use std::any::Any;

/// Base multiplier for Stop Loss.
const BASE_SL_MULTI: f64 = 4.0;

/// Base multiplier for Take Profit.
const BASE_TP_MULTI: f64 = 6.0;

/// # Dynamic Exits Indicator (Regime-Adaptive)
///
/// Computes the absolute dollar *distance* for Take-Profit and Stop-Loss based on the ATR,
/// dynamically scaled by the current Market Regime (Volatility, Trend, and Efficiency).
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
        let regime = ctx.indicator::<MarketRegime>();
        let er = ctx.indicator::<EfficiencyRatio<10, 3>>();

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

            let vol = regime.volatility.get(i).copied().unwrap_or(0.5);
            let trend = regime.trend.get(i).copied().unwrap_or(0.0).abs();
            let efficiency = er.smooth.get(i).copied().unwrap_or(0.5);

            let sl_multi = (BASE_SL_MULTI + (vol - 0.5) * 2.0).clamp(1.2, 3.5);

            let vol_tp_adj = (vol - 0.5) * 1.5;
            let trend_tp_adj = (trend - 0.3) * 2.5;
            let eff_tp_adj = (efficiency - 0.5) * 1.5;

            let tp_multi = (BASE_TP_MULTI + vol_tp_adj + trend_tp_adj + eff_tp_adj).clamp(1.5, 6.0);

            self.sl_distance.push(atr_val * sl_multi);
            self.tp_distance.push(atr_val * tp_multi);
        }
    }

    fn is_computed(&self) -> bool {
        !self.sl_distance.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
