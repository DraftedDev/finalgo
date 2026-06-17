use crate::engine::Context;
use crate::indicator::atr::AvgTrueRange;
use crate::indicator::boll::BollingerBands;
use crate::score::Score;
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
    /// Creates a new [VolatilityScore] instance.
    pub fn new() -> Self {
        Self {
            volatility: 0.0,
            confidence: 0.0,
            computed: false,
        }
    }

    /// Converts the latest value of a positive series into a normalized
    /// 0..1 score relative to its own historical distribution.
    ///
    /// - around the historical mean -> ~0.5
    /// - above the mean -> toward 1.0
    /// - below the mean -> toward 0.0
    #[inline]
    fn relative_component(values: &[f64]) -> f64 {
        let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();

        if finite.len() < 2 {
            return 0.5;
        }

        let mean = finite.iter().sum::<f64>() / finite.len() as f64;
        let var = finite
            .iter()
            .map(|v| {
                let d = *v - mean;
                d * d
            })
            .sum::<f64>()
            / finite.len() as f64;

        let std = var.sqrt();
        if std <= 1e-12 {
            return 0.5;
        }

        let last = *finite.last().unwrap();
        let z = (last - mean) / std;

        // Keep it smooth and centered.
        (0.5 + 0.5 * (z / 2.0).tanh()).clamp(0.0, 1.0)
    }
}

impl Score for VolatilityScore {
    fn name() -> String {
        "volatility".to_string()
    }

    fn compute(&mut self, ctx: Context) {
        let regime_vol = ctx.regime().volatility.clamp(0.0, 1.0);

        let atr = ctx.indicator::<AvgTrueRange<14>>();
        let bb = ctx.indicator::<BollingerBands<20, 2>>();

        let atr_component = Self::relative_component(&atr.norm_atr);
        let bb_component = Self::relative_component(&bb.width);

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
