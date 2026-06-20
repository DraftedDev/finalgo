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

        self.values.reserve(len);

        let mut sum = 0.0;
        let mut valid_count = 0;

        for i in 0..len {
            let v = volume[i];
            let is_safe = Self::safe(v);

            if i > PERIOD {
                let old_idx = i - PERIOD - 1;
                let old_v = volume[old_idx];
                if Self::safe(old_v) {
                    sum -= old_v;
                    valid_count -= 1;
                }
            }

            if i >= PERIOD && valid_count > 0 {
                let avg = sum / valid_count as f64;

                if is_safe && avg > 1e-12 {
                    self.values.push((v / avg).min(5.0));
                } else {
                    self.values.push(if is_safe { 1.0 } else { f64::NAN });
                }
            } else {
                self.values.push(f64::NAN);
            }

            if is_safe {
                sum += v;
                valid_count += 1;
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
