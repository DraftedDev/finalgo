use crate::engine::Context;
use crate::indicator::Indicator;
use std::any::Any;

/// # Efficiency Ratio Indicator
///
/// Measures how efficiently price moves from one point to another.
///
/// High values indicate directional movement with little noise.
/// Low values indicate choppy or mean-reverting price action.
pub struct EfficiencyRatio<const PERIOD: usize, const SMOOTH: usize> {
    /// Raw Efficiency Ratio (ER).
    ///
    /// Computed as:
    ///
    /// ```text
    /// abs(close_t - close_{t-PERIOD})
    /// ---------------------------------
    /// sum(abs(close_i - close_{i-1}))
    /// ```
    ///
    /// Range:
    ///
    /// ```text
    /// [0, 1]
    /// ```
    ///
    /// Interpretation:
    /// - 1.0 = perfectly directional movement
    /// - 0.0 = highly noisy or sideways movement
    pub er: Vec<f64>,

    /// Smoothed Efficiency Ratio.
    ///
    /// Moving average of `er` over `SMOOTH` periods.
    ///
    /// Reduces short-term fluctuations and provides a more stable
    /// estimate of market efficiency.
    ///
    /// Range:
    ///
    /// ```text
    /// [0, 1]
    /// ```
    pub smooth: Vec<f64>,

    /// First derivative of the smoothed Efficiency Ratio.
    ///
    /// Computed as:
    ///
    /// ```text
    /// smooth_t - smooth_{t-1}
    /// ```
    ///
    /// Interpretation:
    /// - positive = efficiency is increasing
    /// - negative = efficiency is decreasing
    /// - near zero = efficiency is stable
    ///
    /// Can be used to detect transitions between trending
    /// and choppy market conditions.
    pub slope: Vec<f64>,
}

impl<const PERIOD: usize, const SMOOTH: usize> EfficiencyRatio<PERIOD, SMOOTH> {
    /// Create a new empty [EfficiencyRatio] instance.
    pub fn new() -> Self {
        Self {
            er: Vec::new(),
            smooth: Vec::new(),
            slope: Vec::new(),
        }
    }

    #[inline]
    fn mean(slice: &[f64]) -> f64 {
        let mut sum = 0.0;
        let mut n = 0;

        for v in slice {
            if v.is_finite() {
                sum += *v;
                n += 1;
            }
        }

        if n == 0 { 0.0 } else { sum / n as f64 }
    }
}

impl<const PERIOD: usize, const SMOOTH: usize> Indicator for EfficiencyRatio<PERIOD, SMOOTH> {
    fn name() -> String {
        format!("er-{}", PERIOD)
    }

    fn compute(&mut self, ctx: Context) {
        let closes = &ctx.data().closes;
        let len = closes.len();

        self.er = vec![f64::NAN; len];
        self.smooth = vec![f64::NAN; len];
        self.slope = vec![f64::NAN; len];

        for i in PERIOD..len {
            let mut path = 0.0;

            for j in (i - PERIOD + 1)..=i {
                path += (closes[j] - closes[j - 1]).abs();
            }

            let net = (closes[i] - closes[i - PERIOD]).abs();

            self.er[i] = if path > 1e-12 {
                (net / path).clamp(0.0, 1.0)
            } else {
                0.0
            };
        }

        for i in (PERIOD + SMOOTH)..len {
            let window = &self.er[i - SMOOTH..i];

            if window.iter().all(|v| v.is_finite()) {
                self.smooth[i] = Self::mean(window).clamp(0.0, 1.0);

                let prev = self.smooth[i - 1];
                if prev.is_finite() {
                    self.slope[i] = self.smooth[i] - prev;
                } else {
                    self.slope[i] = 0.0;
                }
            }
        }
    }

    fn is_computed(&self) -> bool {
        !self.er.is_empty()
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
