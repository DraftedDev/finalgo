use crate::engine::Context;
use crate::indicator::atr::AvgTrueRange;
use crate::indicator::boll::BollingerBands;
use crate::indicator::ema::ExpMovAvg;
use crate::indicator::er::EfficiencyRatio;
use crate::indicator::roc::RateOfChange;
use crate::indicator::rvol::RelativeVolume;
use crate::indicator::swing::SwingStructure;
use crate::math;

/// The regime of the current market.
///
/// Each field represents a different market state attribute.
#[derive(Clone, Debug)]
pub struct Regime {
    /// How trendy the market behaves from -1.0 (bearish) to 0.0 (flat) to 1.0 (bullish).
    pub trend: f64,
    /// How volatile the market is from 0.0 (compressed) to 1.0 (explosive).
    pub volatility: f64,
    /// How structured/aligned swings are from -1.0 (bearish structure) to 1.0 (bullish structure).
    pub structure: f64,
    /// How much participation there is from 0.0 (dead market) to 1.0 (extreme participation).
    pub participation: f64,
}

impl Regime {
    pub fn compute(ctx: Context) -> Self {
        Self {
            trend: compute_trend::<600, 10, 10, 3, 10, 10>(&ctx),
            volatility: compute_volatility::<14, 30, 2>(&ctx),
            structure: compute_structure::<10, 10>(&ctx),
            participation: compute_participation::<20>(&ctx),
        }
    }
}

/// Compute the market trend regime attribute.
fn compute_trend<
    const EMA_PERIOD: usize,
    const ROC_PERIOD: usize,
    const ER_PERIOD: usize,
    const ER_SMOOTH: usize,
    const SWING_LEFT: usize,
    const SWING_RIGHT: usize,
>(
    ctx: &Context,
) -> f64 {
    let ema = ctx.indicator::<ExpMovAvg<EMA_PERIOD>>();
    let roc = ctx.indicator::<RateOfChange<ROC_PERIOD>>();
    let er = ctx.indicator::<EfficiencyRatio<ER_PERIOD, ER_SMOOTH>>();
    let swing = ctx.indicator::<SwingStructure<SWING_LEFT, SWING_RIGHT>>();
    let atr = ctx.indicator::<AvgTrueRange<14>>();

    let current_atr = math::last_finite(&atr.atr).unwrap_or(1.0).max(1e-12);

    let ema_distance = math::last_finite(&ema.distance).unwrap_or(0.0);
    let ema_slope = math::last_finite(&ema.slope).unwrap_or(0.0);

    let ema_bias = (ema_distance / current_atr).clamp(-3.0, 3.0) / 3.0;

    let ema_slope_score = (ema_slope / current_atr).clamp(-1.0, 1.0);

    let roc_last = math::last_finite(&roc.roc).unwrap_or(0.0);
    let roc_score = (roc_last * 20.0).tanh();

    let structure = math::last_finite(&swing.structure).unwrap_or(0.0);
    let structure_strength = math::last_finite(&swing.structure_strength).unwrap_or(0.0);
    let structure_score = (structure * (0.5 + 0.5 * structure_strength)).clamp(-1.0, 1.0);

    let bos = math::last_non_zero(&swing.bos)
        .unwrap_or(0.0)
        .clamp(-1.0, 1.0);
    let choch = math::last_non_zero(&swing.choch)
        .unwrap_or(0.0)
        .clamp(-1.0, 1.0);

    let bos_score = bos.clamp(-1.0, 1.0);

    let choch_penalty = choch.clamp(-1.0, 1.0);

    let raw_trend = 0.30 * ema_bias
        + 0.20 * ema_slope_score
        + 0.20 * roc_score
        + 0.20 * structure_score
        + 0.05 * bos_score
        - 0.05 * choch_penalty;

    let er_smooth = math::last_finite(&er.smooth).unwrap_or(0.5).clamp(0.0, 1.0);
    let chop_penalty = (er_smooth - 1.0).clamp(-1.0, 0.0) * 0.30;

    (raw_trend + chop_penalty).clamp(-1.0, 1.0)
}

/// Computes market volatility as a regime attribute in [0.0, 1.0].
pub fn compute_volatility<const ATR_PERIOD: usize, const BB_PERIOD: usize, const STD_MULTI: i32>(
    ctx: &Context,
) -> f64 {
    let atr = ctx.indicator::<AvgTrueRange<ATR_PERIOD>>();
    let bb = ctx.indicator::<BollingerBands<BB_PERIOD, STD_MULTI>>();

    let atr_norm = math::last_finite(&atr.norm_atr).unwrap_or(0.0);
    let bb_width = math::last_finite(&bb.width).unwrap_or(0.0);

    let atr_score = (atr_norm / 0.04).clamp(0.0, 1.0);

    let width_score = (bb_width / 0.12).clamp(0.0, 1.0);

    let volatility = (atr_score * 0.40 + width_score * 0.60).clamp(0.0, 1.0);

    let vol_smooth = volatility * volatility * (3.0 - 2.0 * volatility);

    vol_smooth.clamp(0.0, 1.0)
}

/// Computes the structure component of the current market regime.
pub fn compute_structure<const LEFT: usize, const RIGHT: usize>(ctx: &Context) -> f64 {
    let swing = ctx.indicator::<SwingStructure<LEFT, RIGHT>>();

    let structure = math::last_finite(&swing.structure)
        .unwrap_or(0.0)
        .clamp(-1.0, 1.0);
    let strength = math::last_finite(&swing.structure_strength)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);

    let bos = math::last_non_zero(&swing.bos)
        .unwrap_or(0.0)
        .clamp(-1.0, 1.0);
    let choch = math::last_non_zero(&swing.choch)
        .unwrap_or(0.0)
        .clamp(-1.0, 1.0);

    let base = structure * (0.40 + 0.60 * strength);

    let bos_influence = bos * 0.30;

    let choch_influence = choch * 0.50;

    let mut final_structure = base + bos_influence + choch_influence;

    if choch.abs() > 0.5 {
        final_structure = (final_structure * 0.30 + choch * 0.70).clamp(-1.0, 1.0);
    }

    final_structure.clamp(-1.0, 1.0)
}

/// Computes participation from Relative Volume.
pub fn compute_participation<const PERIOD: usize>(ctx: &Context) -> f64 {
    let rvol = ctx.indicator::<RelativeVolume<PERIOD>>();
    let values = &rvol.values;

    let mut recent = Vec::new();
    for &v in values.iter().rev() {
        if v.is_finite() {
            recent.push(v);
        }
        if recent.len() == 5 {
            break;
        }
    }

    if recent.is_empty() {
        return 0.5;
    } // Default to neutral if no data

    let avg_rvol = recent.iter().copied().sum::<f64>() / recent.len() as f64;

    let participation = math::sigmoid((avg_rvol - 1.0) * 2.0);

    participation.clamp(0.0, 1.0)
}
