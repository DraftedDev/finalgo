use crate::engine::Context;
use crate::indicator::atr::AvgTrueRange;
use crate::indicator::boll::BollingerBands;
use crate::math;
use crate::score::Score;
use crate::utils::ValueMap;
use std::any::Any;

/// # Volatility Score
///
/// A score representing the market volatility of a stock.
///
/// Requires:
/// - `AvgTrueRange<14>`
/// - `BollingerBands<20, 2>`
pub struct VolatilityScore {
    /// Normalized volatility score.
    ///
    /// Range:
    /// - 0.0 -> extremely quiet / compressed market
    /// - 0.5 -> normal volatility conditions
    /// - 1.0 -> highly volatile / explosive market conditions
    pub volatility: f64,

    /// Confidence in the volatility estimate.
    ///
    /// Range:
    /// - 1.0 -> all volatility measures agree strongly
    /// - 0.5 -> moderate disagreement between signals
    /// - 0.0 -> conflicting or unstable volatility signals
    pub confidence: f64,

    computed: bool,
}

impl VolatilityScore {
    pub const VOLATILITY_KEY: &str = "volatility";
    pub const CONFIDENCE_KEY: &str = "volatility_confidence";

    pub fn new() -> Self {
        Self {
            volatility: 0.0,
            confidence: 0.0,
            computed: false,
        }
    }

    #[inline]
    fn normalize_positive(x: f64, k: f64) -> f64 {
        if !x.is_finite() || x <= 0.0 {
            0.0
        } else {
            (1.0 - (-k * x).exp()).clamp(0.0, 1.0)
        }
    }
}

impl Score for VolatilityScore {
    fn name(&self) -> String {
        "volatility".to_string()
    }

    fn compute(&mut self, ctx: Context) -> ValueMap {
        let regime_vol = ctx.regime().volatility;

        // Use the latest finite ATR normalization.
        let atr = ctx.indicator::<AvgTrueRange<14>>();
        let atr_norm = math::last_finite(&atr.norm_atr).unwrap_or(0.0);

        // Use the latest finite Bollinger width.
        let bb = ctx.indicator::<BollingerBands<20, 2>>();
        let bb_width = math::last_finite(&bb.width).unwrap_or(0.0);

        // Convert both positive volatility proxies into [0, 1].
        let atr_component = Self::normalize_positive(atr_norm, 80.0);
        let bb_component = Self::normalize_positive(bb_width, 25.0);

        // Blend the three sources.
        //
        // Regime gets the most weight because it is already a higher-level summary.
        let volatility =
            (atr_component * 0.35 + bb_component * 0.25 + regime_vol * 0.40).clamp(0.0, 1.0);

        // Confidence is higher when the components agree.
        let spread = (atr_component - bb_component)
            .abs()
            .max((volatility - regime_vol).abs());
        let confidence = (1.0 - spread).clamp(0.0, 1.0);

        self.volatility = volatility;
        self.confidence = confidence;
        self.computed = true;

        ValueMap::new()
            .with(Self::VOLATILITY_KEY, self.volatility)
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
