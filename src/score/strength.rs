use crate::engine::Context;
use crate::indicator::atr::AvgTrueRange;
use crate::indicator::ema::ExpMovAvg;
use crate::indicator::roc::RateOfChange;
use crate::indicator::swing::SwingStructure;
use crate::score::Score;
use crate::utils::ValueMap;
use std::any::Any;

/// # Strength Score
///
/// A score representing the future trend strength of a stock.
///
/// Requires:
/// - `AvgTrueRange<14>`
/// - `ExpMovAvg<20>`
/// - `RateOfChange<10>`
/// - `SwingStructure<5, 5>`
pub struct StrengthScore {
    pub strength: f64,
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
}

impl Score for StrengthScore {
    fn name(&self) -> String {
        "strength".to_string()
    }

    fn compute(&mut self, ctx: Context) -> ValueMap {
        let regime = ctx.regime();

        let atr = ctx.indicator::<AvgTrueRange<14>>();
        let ema = ctx.indicator::<ExpMovAvg<20>>();
        let roc = ctx.indicator::<RateOfChange<10>>();
        let swing = ctx.indicator::<SwingStructure<5, 5>>();

        let len = atr.atr.len().min(ema.distance.len());

        // last valid index
        let i = match (0..len).rposition(|i| {
            atr.atr[i].is_finite() && ema.distance[i].is_finite() && roc.roc[i].is_finite()
        }) {
            Some(i) => i,
            None => {
                self.strength = 0.0;
                self.confidence = 0.0;
                self.computed = true;

                return ValueMap::new()
                    .with(Self::STRENGTH_KEY, self.strength)
                    .with(Self::CONFIDENCE_KEY, self.confidence);
            }
        };

        let trend_component = ema.slope[i].tanh(); // directional trend force
        let momentum_component = roc.roc_z[i].tanh(); // normalized impulse

        let volatility_factor = (atr.norm_atr[i] * 10.0).tanh(); // avoid dominance
        let structure_component = swing.structure_strength[i].tanh();

        let trend_w = 0.35 + 0.35 * regime.trend.abs();
        let momentum_w = 0.25 + 0.25 * regime.volatility;
        let structure_w = 0.25 + 0.35 * regime.structure.abs();
        let volatility_w = 0.10 + 0.25 * regime.volatility;

        let raw_strength = trend_component * trend_w
            + momentum_component * momentum_w
            + structure_component * structure_w
            + volatility_factor * volatility_w;

        // normalize
        let strength = (raw_strength * regime.participation).tanh();

        // confidence = agreement of signals
        let agreement =
            (trend_component.signum() + momentum_component.signum() + structure_component.signum())
                .abs()
                / 3.0;

        let confidence = (agreement * regime.participation).clamp(0.0, 1.0);

        self.strength = strength.clamp(-1.0, 1.0);
        self.confidence = confidence;
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
