use crate::engine::Context;
use crate::indicator::Indicator;
use crate::indicator::atr::AvgTrueRange;
use crate::indicator::boll::BollingerBands;
use crate::indicator::ema::ExpMovAvg;
use crate::indicator::er::EfficiencyRatio;
use crate::indicator::roc::RateOfChange;
use crate::indicator::rvol::RelativeVolume;
use crate::indicator::swing::SwingStructure;
use crate::math;
use std::any::Any;

/// # Market Regime Indicator
///
/// Computes the macro-environmental state of the market for every bar in history.
/// Tracks Trend, Volatility, Structure, and Participation.
#[derive(Clone, Debug, Default)]
pub struct MarketRegime {
    /// How trendy the market behaves from -1.0 (bearish) to 0.0 (flat) to 1.0 (bullish).
    pub trend: Vec<f64>,
    /// How volatile the market is from 0.0 (compressed) to 1.0 (explosive).
    pub volatility: Vec<f64>,
    /// How structured/aligned swings are from -1.0 (bearish structure) to 1.0 (bullish structure).
    pub structure: Vec<f64>,
    /// How much participation there is from 0.0 (dead market) to 1.0 (extreme participation).
    pub participation: Vec<f64>,

    computed: bool,
}

impl MarketRegime {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Indicator for MarketRegime {
    fn name() -> String {
        "regime".to_string()
    }

    fn compute(&mut self, ctx: Context) {
        let data = ctx.data();
        let len = data.closes.len();

        let ema = ctx.indicator::<ExpMovAvg<100>>();
        let roc = ctx.indicator::<RateOfChange<10>>();
        let er = ctx.indicator::<EfficiencyRatio<10, 3>>();
        let swing = ctx.indicator::<SwingStructure<5, 10>>();
        let atr = ctx.indicator::<AvgTrueRange<14>>();
        let bb = ctx.indicator::<BollingerBands<20, 2>>();
        let rvol = ctx.indicator::<RelativeVolume<20>>();

        self.trend = Vec::with_capacity(len);
        self.volatility = Vec::with_capacity(len);
        self.structure = Vec::with_capacity(len);
        self.participation = Vec::with_capacity(len);

        let mut current_bos = 0.0;
        let mut current_choch = 0.0;
        let mut prev_atr = 1.0;

        let mut rvol_window = [1.0; 5];
        let mut rvol_idx = 0;
        let mut rvol_sum = 5.0;
        let mut rvol_count = 0;

        for i in 0..len {
            if swing.bos[i] != 0.0 {
                current_bos = swing.bos[i];
            }
            if swing.choch[i] != 0.0 {
                current_choch = swing.choch[i];
            }

            let raw_atr = atr.atr[i];
            let current_atr = if raw_atr.is_finite() && raw_atr > 1e-12 {
                raw_atr
            } else {
                prev_atr
            };
            prev_atr = current_atr;

            let ema_dist = ema.distance[i];
            let ema_slope = ema.slope[i];

            let ema_bias = if ema_dist.is_finite() {
                (ema_dist / current_atr).clamp(-3.0, 3.0) / 3.0
            } else {
                0.0
            };
            let ema_slope_score = if ema_slope.is_finite() {
                (ema_slope / current_atr).clamp(-1.0, 1.0)
            } else {
                0.0
            };

            let roc_val = roc.roc[i];
            let roc_score = if roc_val.is_finite() {
                (roc_val * 20.0).tanh()
            } else {
                0.0
            };

            let struct_val = swing.structure[i];
            let struct_str = swing.structure_strength[i];
            let structure_score = if struct_val.is_finite() && struct_str.is_finite() {
                (struct_val * (0.5 + 0.5 * struct_str)).clamp(-1.0, 1.0)
            } else {
                0.0
            };

            let bos_score = current_bos.clamp(-1.0, 1.0);
            let choch_penalty = current_choch.clamp(-1.0, 1.0);

            let raw_trend = 0.30 * ema_bias
                + 0.20 * ema_slope_score
                + 0.20 * roc_score
                + 0.20 * structure_score
                + 0.05 * bos_score
                - 0.05 * choch_penalty;

            let er_smooth = er.smooth[i];
            let chop_penalty = if er_smooth.is_finite() {
                (er_smooth - 1.0).clamp(-1.0, 0.0) * 0.30
            } else {
                0.0
            };

            let trend = (raw_trend + chop_penalty).clamp(-1.0, 1.0);
            self.trend.push(trend);

            let atr_norm = atr.norm_atr[i];
            let bb_width = bb.width[i];

            let atr_score = if atr_norm.is_finite() {
                (atr_norm / 0.04).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let width_score = if bb_width.is_finite() {
                (bb_width / 0.12).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let volatility_raw = (atr_score * 0.40 + width_score * 0.60).clamp(0.0, 1.0);
            let volatility = volatility_raw * volatility_raw * (3.0 - 2.0 * volatility_raw);

            self.volatility.push(volatility);

            let base = if struct_val.is_finite() && struct_str.is_finite() {
                struct_val * (0.40 + 0.60 * struct_str)
            } else {
                0.0
            };

            let bos_influence = current_bos * 0.30;
            let choch_influence = current_choch * 0.50;

            let mut final_structure = base + bos_influence + choch_influence;

            if current_choch.abs() > 0.5 {
                final_structure = (final_structure * 0.30 + current_choch * 0.70).clamp(-1.0, 1.0);
            } else {
                final_structure = final_structure.clamp(-1.0, 1.0);
            }
            self.structure.push(final_structure);

            let current_rvol = rvol.values[i];
            if current_rvol.is_finite() {
                rvol_sum -= rvol_window[rvol_idx];
                rvol_window[rvol_idx] = current_rvol;
                rvol_sum += current_rvol;
                rvol_idx = (rvol_idx + 1) % 5;
                if rvol_count < 5 {
                    rvol_count += 1;
                }
            }

            let avg_rvol = if rvol_count > 0 {
                rvol_sum / rvol_count as f64
            } else {
                1.0
            };

            let participation = math::sigmoid((avg_rvol - 1.0) * 2.0).clamp(0.0, 1.0);

            self.participation.push(participation);
        }

        self.computed = true;
    }

    fn is_computed(&self) -> bool {
        self.computed
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
