use crate::engine::Context;
use crate::indicator::ema::ExpMovAvg;
use crate::indicator::er::EfficiencyRatio;
use crate::indicator::roc::RateOfChange;
use crate::indicator::swing::SwingStructure;
use crate::math;
use crate::score::Score;
use crate::score::trend::TrendScore;
use crate::utils::ValueMap;
use std::any::Any;

/// # Strength Score
///
/// Represents how strong the current trend is, regardless of direction.
///
/// Requires:
/// - `ExpMovAvg<600>`
/// - `SwingStructure<5, 10>`
/// - `RateOfChange<10>`
/// - `EfficiencyRatio<10, 3>`
pub struct StrengthScore {
    /// Strength of the current trend.
    ///
    /// Range:
    /// - 0.0 -> no meaningful trend / weak or choppy movement
    /// - 1.0 -> very strong, sustained directional trend
    pub strength: f64,

    /// Confidence in the strength estimate.
    ///
    /// Range:
    /// - 0.0 -> unreliable / conflicting signals
    /// - 1.0 -> highly consistent and trustworthy trend conditions
    pub confidence: f64,

    computed: bool,
}

impl StrengthScore {
    pub const STRENGTH_KEY: &str = "strength";
    pub const CONFIDENCE_KEY: &str = "strength_confidence";

    pub fn new() -> Self {
        Self {
            strength: 0.0,
            confidence: 0.0,
            computed: false,
        }
    }

    #[inline]
    fn normalize_ratio(x: f64, scale: f64) -> f64 {
        if !x.is_finite() || !scale.is_finite() || scale.abs() <= 1e-12 {
            return 0.0;
        }

        (x.abs() * scale).tanh().clamp(0.0, 1.0)
    }

    #[inline]
    fn mean(values: &[f64]) -> f64 {
        let mut sum = 0.0;
        let mut count = 0usize;

        for &v in values {
            if v.is_finite() {
                sum += v;
                count += 1;
            }
        }

        if count == 0 { 0.0 } else { sum / count as f64 }
    }

    #[inline]
    fn spread(values: &[f64]) -> f64 {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for &v in values {
            if v.is_finite() {
                min = min.min(v);
                max = max.max(v);
            }
        }

        if !min.is_finite() || !max.is_finite() {
            0.0
        } else {
            (max - min).clamp(0.0, 1.0)
        }
    }
}

impl Score for StrengthScore {
    fn name() -> String {
        "strength".to_string()
    }

    fn compute(&mut self, ctx: Context) -> ValueMap {
        let trend = ctx.score::<TrendScore>();
        let regime = ctx.regime();

        let ema = ctx.indicator::<ExpMovAvg<600>>();
        let swing = ctx.indicator::<SwingStructure<5, 10>>();
        let roc = ctx.indicator::<RateOfChange<10>>();
        let er = ctx.indicator::<EfficiencyRatio<10, 3>>();

        let close = ctx
            .data()
            .closes
            .last()
            .copied()
            .unwrap_or(0.0)
            .abs()
            .max(1e-12);

        // Pure magnitude inputs, no confidence mixed in here.
        let trend_mag = trend.direction.abs().clamp(0.0, 1.0);
        let regime_trend = regime.trend.abs().clamp(0.0, 1.0);

        let ema_distance = Self::normalize_ratio(
            math::last_finite(&ema.distance).unwrap_or(0.0),
            25.0 / close,
        );

        let ema_slope =
            Self::normalize_ratio(math::last_finite(&ema.slope).unwrap_or(0.0), 50.0 / close);

        let roc_strength =
            Self::normalize_ratio(math::last_finite(&roc.roc_abs).unwrap_or(0.0), 20.0);

        let structure = math::last_finite(&swing.structure_strength)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);

        let efficiency = math::last_finite(&er.smooth).unwrap_or(0.0).clamp(0.0, 1.0);

        let trend_component = 0.55 * trend_mag + 0.45 * regime_trend;

        let impulse_component = 0.45 * roc_strength + 0.30 * ema_slope + 0.25 * ema_distance;

        let structure_component = 0.60 * structure + 0.40 * efficiency;

        self.strength =
            (0.38 * trend_component + 0.32 * impulse_component + 0.30 * structure_component)
                .clamp(0.0, 1.0);

        let conf_values = [
            trend.confidence.clamp(0.0, 1.0),
            structure,
            efficiency,
            regime_trend,
            1.0 - (trend_mag - regime_trend).abs(),
        ];

        let conf_base = Self::mean(&conf_values);
        let agreement =
            1.0 - Self::spread(&[trend_component, impulse_component, structure_component]);

        self.confidence = (conf_base * (0.60 + 0.40 * agreement)).clamp(0.0, 1.0);

        self.computed = true;

        ValueMap::new()
            .with(Self::STRENGTH_KEY, self.strength)
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
