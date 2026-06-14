use crate::engine::Context;
use crate::indicator::atr::AvgTrueRange;
use crate::indicator::ema::ExpMovAvg;
use crate::indicator::er::EfficiencyRatio;
use crate::indicator::roc::RateOfChange;
use crate::indicator::rsi::RelStrengthIdx;
use crate::indicator::swing::SwingStructure;
use crate::math;
use crate::score::Score;
use crate::utils::ValueMap;
use std::any::Any;

/// # Trend Score
///
/// A score representing the future trend prediction of a stock.
///
/// Requires:
/// - `ExpMovAvg<600>`
/// - `SwingStructure<5, 10>`
/// - `RateOfChange<10>`
/// - `EfficiencyRatio<10, 3>`
pub struct TrendScore {
    /// Final directional trend estimate.
    pub direction: f64,

    /// Confidence in the trend estimate.
    pub confidence: f64,

    computed: bool,
}

impl TrendScore {
    pub const DIRECTION_KEY: &'static str = "trend_direction";
    pub const CONFIDENCE_KEY: &'static str = "trend_confidence";

    pub fn new() -> Self {
        Self {
            direction: 0.0,
            confidence: 0.0,
            computed: false,
        }
    }
}
impl Score for TrendScore {
    fn name() -> String {
        "trend".to_string()
    }

    fn compute(&mut self, ctx: Context) -> ValueMap {
        let regime = ctx.regime();
        let ema = ctx.indicator::<ExpMovAvg<100>>();
        let swing = ctx.indicator::<SwingStructure<5, 10>>();
        let roc = ctx.indicator::<RateOfChange<10>>();
        let er = ctx.indicator::<EfficiencyRatio<10, 3>>();
        let rsi = ctx.indicator::<RelStrengthIdx<14>>();
        let atr = ctx.indicator::<AvgTrueRange<14>>();

        let current_atr = math::last_finite(&atr.atr).unwrap_or(1.0).max(1e-12);

        let ema_slope = math::last_finite(&ema.slope).unwrap_or(0.0);
        let macro_trend = (ema_slope / current_atr * 5.0).tanh().clamp(-1.0, 1.0);

        let structure = math::last_finite(&swing.structure).unwrap_or(0.0);
        let structure_strength = math::last_finite(&swing.structure_strength).unwrap_or(0.0);
        let recent_bos = math::last_non_zero(&swing.bos)
            .unwrap_or(0.0)
            .clamp(-1.0, 1.0);
        let recent_choch = math::last_non_zero(&swing.choch)
            .unwrap_or(0.0)
            .clamp(-1.0, 1.0);

        let struct_trend = structure.clamp(-1.0, 1.0);
        let struct_shift = (recent_bos * 0.7 + recent_choch * 0.3).clamp(-1.0, 1.0);

        let structure_dir = (struct_trend * 0.60 + struct_shift * 0.40).clamp(-1.0, 1.0);

        let roc_value = math::last_finite(&roc.roc).unwrap_or(0.0);
        let roc_dir = (roc_value * 20.0).tanh();

        let rsi_val = math::last_finite(&rsi.rsi).unwrap_or(0.0);

        let exhaustion_signal = if rsi_val.abs() > 0.5 {
            -rsi_val.signum() * (rsi_val.abs() - 0.5) * 2.0
        } else {
            0.0
        };

        let short_term_trigger = (roc_dir * 0.6 + exhaustion_signal * 0.4).clamp(-1.0, 1.0);

        let core_dir = (macro_trend * 0.3 + structure_dir * 0.7).clamp(-1.0, 1.0);

        let amplified_core = core_dir.signum() * core_dir.abs().powf(0.65);

        let alignment = amplified_core * short_term_trigger;

        let direction = if amplified_core.abs() > 0.10 {
            let mag = amplified_core.abs() + alignment * 0.40;
            amplified_core.signum() * mag.clamp(0.0, 1.0)
        } else {
            short_term_trigger * 0.6
        };

        let direction = direction.clamp(-1.0, 1.0);

        let regime_vol = regime.volatility.clamp(0.0, 1.0);
        let er_value = math::last_finite(&er.smooth).unwrap_or(0.0).clamp(0.0, 1.0);

        let dominant_sign =
            if structure_dir.signum() == macro_trend.signum() && structure_dir.abs() > 0.1 {
                structure_dir.signum()
            } else {
                0.0
            };

        let agreement_score = if dominant_sign != 0.0 {
            let align_count = [
                macro_trend.signum(),
                structure_dir.signum(),
                short_term_trigger.signum(),
            ]
            .iter()
            .filter(|&&s| s == dominant_sign)
            .count();
            align_count as f64 / 3.0
        } else {
            0.25
        };

        let core_energy = amplified_core.abs();
        let market_clarity = er_value * 0.6 + structure_strength.clamp(0.0, 1.0) * 0.4;

        let vol_distance = (regime_vol - 0.5).abs();
        let vol_penalty = (1.0 - vol_distance * 1.5).clamp(0.0, 1.0);

        let confidence = (agreement_score * 0.35
            + core_energy * 0.25
            + market_clarity * 0.25
            + vol_penalty * 0.15)
            .clamp(0.0, 1.0);

        self.direction = direction;
        self.confidence = confidence;
        self.computed = true;

        ValueMap::new()
            .with(Self::DIRECTION_KEY, self.direction)
            .with(Self::CONFIDENCE_KEY, self.confidence)
    }

    fn is_computed(&self) -> bool {
        self.computed
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
