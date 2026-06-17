use crate::engine::Context;
use crate::indicator::Indicator;
use std::any::Any;

/// # Relative Volume
///
/// Measures how current volume compares to its recent average baseline.
pub struct RelativeVolume<const PERIOD: usize> {
    /// Relative volume values over time.
    ///
    /// Computed as:
    ///
    /// ```text
    /// RVOL = volume / SMA(volume over PERIOD)
    /// ```
    ///
    /// Interpretation:
    /// - `1.0` -> normal volume (at baseline)
    /// - `> 1.0` -> elevated trading activity
    /// - `< 1.0` -> reduced trading activity
    ///
    /// Values are softly clamped at a maximum of 5.0 to reduce extreme spikes
    /// from distorting downstream scoring logic.
    pub values: Vec<f64>,
}

impl<const PERIOD: usize> RelativeVolume<PERIOD> {
    /// Create a new empty [RelativeVolume] instance.
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    #[inline]
    fn safe(v: f64) -> bool {
        v.is_finite() && v >= 0.0
    }
}

impl<const PERIOD: usize> Indicator for RelativeVolume<PERIOD> {
    fn name() -> String {
        format!("rvol-{}", PERIOD)
    }

    fn compute(&mut self, ctx: Context) {
        let volume = &ctx.data().volumes;
        let len = volume.len();

        assert!(len >= PERIOD, "Must have at least {PERIOD} samples");

        self.values = vec![f64::NAN; len];

        let mut sum = 0.0;

        // build initial window safely
        for i in 0..len {
            let v = volume[i];

            if !Self::safe(v) {
                continue;
            }

            // build rolling window
            if i < PERIOD {
                sum += v;
                continue;
            }

            let avg = sum / PERIOD as f64;

            if avg > 1e-12 {
                let rvol = v / avg;

                // soft clamp to reduce explosion noise
                self.values[i] = rvol.min(5.0);
            } else {
                self.values[i] = 1.0;
            }

            sum += v;

            let old = volume[i - PERIOD];
            if Self::safe(old) {
                sum -= old;
            }
        }
    }

    fn is_computed(&self) -> bool {
        !self.values.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
