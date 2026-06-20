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

/// Rolling window for volatility Z-scores (approx 6 months of trading days)
const VOL_WINDOW: usize = 120;

/// Rolling window for participation (1 week)
const PART_WINDOW: usize = 5;

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
    /// Create a new [MarketRegime] instance.
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

        let mut atr_ring = [0.0; VOL_WINDOW];
        let mut bb_ring = [0.0; VOL_WINDOW];
        let mut ring_idx = 0;

        let mut atr_sum = 0.0;
        let mut atr_sq = 0.0;
        let mut atr_count = 0;
        let mut bb_sum = 0.0;
        let mut bb_sq = 0.0;
        let mut bb_count = 0;

        let mut rvol_ring = [1.0; PART_WINDOW];
        let mut rvol_idx = 0;
        let mut rvol_sum = PART_WINDOW as f64;
        let mut rvol_count = 0;

        let mut smoothed_trend = 0.0;
        let trend_alpha = 0.33;
        let mut prev_atr = 1.0;

        for i in 0..len {
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
                (ema_slope / current_atr).clamp(-1.5, 1.5) / 1.5
            } else {
                0.0
            };

            let roc_val = roc.roc[i];
            let roc_score = if roc_val.is_finite() {
                (roc_val * 15.0).tanh()
            } else {
                0.0
            };

            let struct_val = swing.structure[i];
            let struct_str = swing.structure_strength[i];
            let structure_score = if struct_val.is_finite() && struct_str.is_finite() {
                struct_val * (0.5 + 0.5 * struct_str)
            } else {
                0.0
            };

            let er_smooth = er.smooth[i];

            let chop_multiplier = if er_smooth.is_finite() && er_smooth < 0.4 {
                0.5
            } else {
                1.0
            };

            let raw_trend = (ema_bias * 0.35
                + ema_slope_score * 0.25
                + roc_score * 0.15
                + structure_score * 0.25)
                * chop_multiplier;

            smoothed_trend = smoothed_trend * (1.0 - trend_alpha) + raw_trend * trend_alpha;

            self.trend.push(smoothed_trend.clamp(-1.0, 1.0));

            let norm_atr = atr.norm_atr[i];
            let bb_width = bb.width[i];

            if norm_atr.is_finite() {
                let old = atr_ring[ring_idx];
                atr_sum -= old;
                atr_sq -= old * old;
                atr_ring[ring_idx] = norm_atr;
                atr_sum += norm_atr;
                atr_sq += norm_atr * norm_atr;
                if atr_count < VOL_WINDOW {
                    atr_count += 1;
                }
            }

            if bb_width.is_finite() {
                let old = bb_ring[ring_idx];
                bb_sum -= old;
                bb_sq -= old * old;
                bb_ring[ring_idx] = bb_width;
                bb_sum += bb_width;
                bb_sq += bb_width * bb_width;
                if bb_count < VOL_WINDOW {
                    bb_count += 1;
                }
            }

            ring_idx = (ring_idx + 1) % VOL_WINDOW;

            let atr_mean = if atr_count > 0 {
                atr_sum / atr_count as f64
            } else {
                0.02
            };
            let atr_var = if atr_count > 1 {
                (atr_sq / atr_count as f64) - (atr_mean * atr_mean)
            } else {
                0.0
            };
            let atr_std = atr_var.max(0.0).sqrt();
            let atr_z = if atr_std > 1e-6 {
                (norm_atr - atr_mean) / atr_std
            } else {
                0.0
            };

            let bb_mean = if bb_count > 0 {
                bb_sum / bb_count as f64
            } else {
                0.05
            };
            let bb_var = if bb_count > 1 {
                (bb_sq / bb_count as f64) - (bb_mean * bb_mean)
            } else {
                0.0
            };
            let bb_std = bb_var.max(0.0).sqrt();
            let bb_z = if bb_std > 1e-6 {
                (bb_width - bb_mean) / bb_std
            } else {
                0.0
            };

            let atr_score = (0.5 + 0.5 * (atr_z / 2.0).tanh()).clamp(0.0, 1.0);
            let bb_score = (0.5 + 0.5 * (bb_z / 2.0).tanh()).clamp(0.0, 1.0);

            let volatility_raw = (atr_score * 0.5 + bb_score * 0.5).clamp(0.0, 1.0);
            let volatility = volatility_raw * volatility_raw * (3.0 - 2.0 * volatility_raw);
            self.volatility.push(volatility);

            let final_structure = if struct_val.is_finite() && struct_str.is_finite() {
                (struct_val * (0.4 + 0.6 * struct_str)).clamp(-1.0, 1.0)
            } else {
                0.0
            };
            self.structure.push(final_structure);

            let current_rvol = rvol.values[i];
            if current_rvol.is_finite() {
                rvol_sum -= rvol_ring[rvol_idx];
                rvol_ring[rvol_idx] = current_rvol;
                rvol_sum += current_rvol;
                rvol_idx = (rvol_idx + 1) % PART_WINDOW;
                if rvol_count < PART_WINDOW {
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
