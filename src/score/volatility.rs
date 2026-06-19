use crate::engine::Context;
use crate::indicator::atr::AvgTrueRange;
use crate::indicator::boll::BollingerBands;
use crate::indicator::regime::MarketRegime;
use crate::score::Score;
use std::any::Any;

/// # Volatility Score
///
/// A score representing the market volatility of a stock.
///
/// Requires:
/// - `MarketRegime`
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
    /// Creates a new [VolatilityScore] instance.
    pub fn new() -> Self {
        Self {
            volatility: 0.0,
            confidence: 0.0,
            computed: false,
        }
    }

    #[inline]
    fn relative_component(values: &[f64], lookback: usize) -> f64 {
        let len = values.len();
        if len == 0 {
            return 0.5;
        }

        let start = len.saturating_sub(lookback);
        let window = &values[start..];

        let mut count = 0;
        let mut sum = 0.0;
        let mut last_valid = f64::NAN;

        for &v in window {
            if v.is_finite() {
                sum += v;
                count += 1;
                last_valid = v;
            }
        }

        if count < 2 || !last_valid.is_finite() {
            return 0.5;
        }

        let mean = sum / count as f64;

        let mut var_sum = 0.0;
        for &v in window {
            if v.is_finite() {
                let d = v - mean;
                var_sum += d * d;
            }
        }

        let std = (var_sum / count as f64).sqrt();
        if std <= 1e-12 {
            return 0.5;
        }

        let z = (last_valid - mean) / std;

        (0.5 + 0.5 * (z / 2.0).tanh()).clamp(0.0, 1.0)
    }
}

impl Score for VolatilityScore {
    fn name() -> String {
        "volatility".to_string()
    }

    fn compute(&mut self, ctx: Context) {
        let regime = ctx.indicator::<MarketRegime>();
        let regime_vol = *regime.volatility.last().unwrap_or(&0.5);

        let atr = ctx.indicator::<AvgTrueRange<14>>();
        let bb = ctx.indicator::<BollingerBands<20, 2>>();

        let atr_component = Self::relative_component(&atr.norm_atr, 100);
        let bb_component = Self::relative_component(&bb.width, 100);

        let volatility =
            (atr_component * 0.40 + bb_component * 0.35 + regime_vol * 0.25).clamp(0.0, 1.0);

        let disagreement = ((atr_component - bb_component).abs()
            + (atr_component - regime_vol).abs()
            + (bb_component - regime_vol).abs())
            / 3.0;

        let confidence = (1.0 - disagreement).clamp(0.0, 1.0);

        self.volatility = volatility;
        self.confidence = confidence;
        self.computed = true;
    }

    fn is_computed(&self) -> bool {
        self.computed
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
