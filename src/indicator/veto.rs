use crate::engine::Context;
use crate::indicator::Indicator;
use crate::indicator::atr::AvgTrueRange;
use crate::indicator::ema::ExpMovAvg;
use std::any::Any;

/// # Macro Veto Indicator
///
/// Evaluates the long-term structural trend of the asset to determine if counter-trend trades should be strictly vetoed.
pub struct MacroVeto {
    /// If true, SHORT signals are blocked because the asset is in a confirmed uptrend.
    pub veto_shorts: Vec<bool>,
    /// If true, LONG signals are blocked because the asset is in a confirmed downtrend.
    pub veto_longs: Vec<bool>,
}

impl MacroVeto {
    pub fn new() -> Self {
        Self {
            veto_shorts: Vec::new(),
            veto_longs: Vec::new(),
        }
    }
}

impl Indicator for MacroVeto {
    fn name() -> String {
        "macro_veto".to_string()
    }

    fn compute(&mut self, ctx: Context) {
        let data = ctx.data();
        let len = data.closes.len();

        let ema = ctx.indicator::<ExpMovAvg<100>>();
        let atr = ctx.indicator::<AvgTrueRange<14>>();

        self.veto_shorts = vec![false; len];
        self.veto_longs = vec![false; len];

        let mut prev_atr = 1.0;

        for i in 0..len {
            let ema_slope = ema.slope[i];
            let raw_atr = atr.atr[i];

            let current_atr = if raw_atr.is_finite() && raw_atr > 1e-12 {
                raw_atr
            } else {
                prev_atr
            };
            prev_atr = current_atr;

            if !ema_slope.is_finite() {
                continue;
            }

            let norm_slope = ema_slope / current_atr;

            let is_uptrend = norm_slope > 0.02;
            let is_downtrend = norm_slope < -0.02;

            self.veto_shorts[i] = is_uptrend;
            self.veto_longs[i] = is_downtrend;
        }
    }

    fn is_computed(&self) -> bool {
        !self.veto_shorts.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
