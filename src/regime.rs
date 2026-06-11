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
///
/// Range:
/// - -1.0 => strong bearish trend
/// -  0.0 => flat / unclear
/// -  1.0 => strong bullish trend
///
/// Requires:
/// - EMA
/// - ROC
/// - Efficiency Ratio
/// - Swing Structure
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
    let close = *ctx.data().closes.last().unwrap_or(&1.0);
    let close = close.max(1e-12);

    let ema = ctx.indicator::<ExpMovAvg<EMA_PERIOD>>();
    let roc = ctx.indicator::<RateOfChange<ROC_PERIOD>>();
    let er = ctx.indicator::<EfficiencyRatio<ER_PERIOD, ER_SMOOTH>>();
    let swing = ctx.indicator::<SwingStructure<SWING_LEFT, SWING_RIGHT>>();

    let ema_slope = *ema.slope.last().unwrap_or(&0.0);
    let ema_distance = *ema.distance.last().unwrap_or(&0.0);

    let roc_last = *roc.roc.last().unwrap_or(&0.0);
    let er_smooth = *er.smooth.last().unwrap_or(&0.0);

    let structure = *swing.structure.last().unwrap_or(&0.0);
    let structure_strength = *swing.structure_strength.last().unwrap_or(&0.0);

    let bos = *swing.bos.last().unwrap_or(&0.0);
    let choch = *swing.choch.last().unwrap_or(&0.0);

    // EMA distance is scaled by price so it becomes comparable across assets.
    let ema_bias = ((ema_distance / close) * 25.0).tanh();

    // EMA slope is small in raw terms, so it needs stronger scaling.
    let ema_slope_score = ((ema_slope / close) * 250.0).tanh();

    // ROC is already normalized, but still benefits from squashing.
    let roc_score = (roc_last * 20.0).tanh();

    // Structure is already signed.
    // Structure strength acts as a confidence multiplier.
    let structure_score = (structure * (0.5 + 0.5 * structure_strength)).clamp(-1.0, 1.0);

    // BOS reinforces trend continuation.
    let bos_score = bos.clamp(-1.0, 1.0);

    // CHoCH usually signals weakening trend / possible reversal.
    // Subtracting it makes the trend score less bullish if CHoCH is bullish,
    // and less bearish if CHoCH is bearish.
    let choch_score = choch.clamp(-1.0, 1.0);

    // --- Combine ---
    let raw_trend = 0.28 * ema_bias
        + 0.22 * ema_slope_score
        + 0.20 * roc_score
        + 0.18 * structure_score
        + 0.08 * bos_score
        - 0.04 * choch_score;

    // Efficiency ratio acts as a confidence gate:
    // choppy markets reduce trend confidence.
    let confidence = (0.35 + 0.65 * er_smooth.clamp(0.0, 1.0)).clamp(0.0, 1.0);

    (raw_trend * confidence).clamp(-1.0, 1.0)
}

/// Computes market volatility as a regime attribute in [0.0, 1.0].
///
/// Range:
/// - 0.0 => compressed / quiet market
/// - 1.0 => explosive / highly volatile market
///
/// Requires:
/// - ATR
/// - Bollinger Bands
pub fn compute_volatility<const ATR_PERIOD: usize, const BB_PERIOD: usize, const STD_MULTI: i32>(
    ctx: &Context,
) -> f64 {
    let atr = ctx.indicator::<AvgTrueRange<ATR_PERIOD>>();
    let bb = ctx.indicator::<BollingerBands<BB_PERIOD, STD_MULTI>>();

    let atr_norm = math::last_finite(&atr.norm_atr).unwrap_or(0.0);
    let bb_width = math::last_finite(&bb.width).unwrap_or(0.0);

    // ATR is the main driver.
    // Width confirms compression/expansion state.
    let atr_score = math::saturate_unit(atr_norm, 0.03);
    let width_score = math::saturate_unit(bb_width, 0.08);

    let volatility = 0.65 * atr_score + 0.35 * width_score;

    volatility.clamp(0.0, 1.0)
}

/// Computes the structure component of the current market regime.
///
/// Output:
/// - -1.0 => bearish structure
/// -  0.0 => mixed / unclear structure
/// -  1.0 => bullish structure
///
/// Requires:
/// - Swing Structure
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

    // BOS confirms continuation, CHoCH represents structural shift.
    // Both are already signed, so they can be blended directly.
    let event_pressure = (0.65 * bos + 0.35 * choch).clamp(-1.0, 1.0);

    // Stronger structure should have more influence than weak structure.
    let base = structure * (0.55 + 0.45 * strength);

    // Event pressure nudges the structure score toward the latest break/reversal state.
    (base + event_pressure * 0.20).clamp(-1.0, 1.0)
}

/// Computes participation from Relative Volume.
///
/// Output range:
/// - 0.0 = dead / low participation
/// - 1.0 = extreme participation
///
/// A value around 1.0 RVOL maps to ~0.5 participation.
/// Values above 1.0 move toward 1.0.
/// Values below 1.0 move toward 0.0.
///
/// Requires:
/// - Relative Volume
pub fn compute_participation<const PERIOD: usize>(ctx: &Context) -> f64 {
    let rvol = ctx.indicator::<RelativeVolume<PERIOD>>();
    let values = &rvol.values;

    let mut recent = Vec::new();

    for &v in values.iter().rev() {
        if v.is_finite() {
            recent.push(v);
        }

        if recent.len() == 3 {
            break;
        }
    }

    if recent.is_empty() {
        return 0.0;
    }

    let avg_rvol = recent.iter().copied().sum::<f64>() / recent.len() as f64;

    // Center at RVOL = 1.0 (normal participation).
    // Steeper multiplier means stronger separation between quiet and active markets.
    let participation = math::sigmoid((avg_rvol - 1.0) * 2.5);

    participation.clamp(0.0, 1.0)
}
