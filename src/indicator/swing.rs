use crate::engine::Context;
use crate::indicator::Indicator;
use std::any::Any;

/// # SwingStructure Indicator
///
/// Captures market structure via swing highs/lows, trend bias,
/// and structural events (BOS / CHoCH) using a robust state machine.
pub struct SwingStructure<const LEFT: usize, const RIGHT: usize> {
    pub swing_highs: Vec<f64>,
    pub swing_lows: Vec<f64>,
    pub structure: Vec<f64>,
    pub structure_strength: Vec<f64>,
    pub bos: Vec<f64>,
    pub choch: Vec<f64>,
}

impl<const LEFT: usize, const RIGHT: usize> SwingStructure<LEFT, RIGHT> {
    /// Create a new [SwingStructure] instance.
    pub fn new() -> Self {
        assert!(LEFT > 0 && RIGHT > 0, "LEFT and RIGHT must be > 0");

        Self {
            swing_highs: Vec::new(),
            swing_lows: Vec::new(),
            structure: Vec::new(),
            structure_strength: Vec::new(),
            bos: Vec::new(),
            choch: Vec::new(),
        }
    }

    #[inline]
    fn is_unique_max(window: &[f64], candidate: f64, max_duplicates: usize) -> bool {
        if !candidate.is_finite() {
            return false;
        }

        let mut count = 0;

        for &v in window {
            if !v.is_finite() {
                return false;
            }

            if v > candidate + 1e-9 {
                return false;
            }

            if (v - candidate).abs() <= 1e-9 {
                count += 1;
            }
        }
        count <= max_duplicates
    }

    #[inline]
    fn is_unique_min(window: &[f64], candidate: f64, max_duplicates: usize) -> bool {
        if !candidate.is_finite() {
            return false;
        }

        let mut count = 0;

        for &v in window {
            if !v.is_finite() {
                return false;
            }

            if v < candidate - 1e-9 {
                return false;
            }

            if (v - candidate).abs() <= 1e-9 {
                count += 1;
            }
        }
        count <= max_duplicates
    }
}

impl<const LEFT: usize, const RIGHT: usize> Indicator for SwingStructure<LEFT, RIGHT> {
    fn name() -> String {
        format!("swing-{}-{}", LEFT, RIGHT)
    }

    fn compute(&mut self, ctx: Context) {
        let data = ctx.data();
        let highs = &data.highs;
        let lows = &data.lows;
        let closes = &data.closes;

        let len = closes.len();

        assert!(
            len > LEFT + RIGHT,
            "Must have at least {LEFT} + {RIGHT} samples"
        );

        self.swing_highs.resize(len, f64::NAN);
        self.swing_lows.resize(len, f64::NAN);
        self.structure.resize(len, 0.0);
        self.structure_strength.resize(len, 0.0);
        self.bos.resize(len, 0.0);
        self.choch.resize(len, 0.0);

        for i in LEFT..(len - RIGHT) {
            let high_window = &highs[i - LEFT..=i + RIGHT];
            let low_window = &lows[i - LEFT..=i + RIGHT];

            if Self::is_unique_max(high_window, highs[i], 3) {
                self.swing_highs[i] = highs[i];
            }

            if Self::is_unique_min(low_window, lows[i], 3) {
                self.swing_lows[i] = lows[i];
            }
        }

        let mut last_high_1: Option<(usize, f64)> = None;
        let mut last_high_2: Option<(usize, f64)> = None;
        let mut last_low_1: Option<(usize, f64)> = None;
        let mut last_low_2: Option<(usize, f64)> = None;

        let mut is_above_last_high = false;
        let mut is_below_last_low = false;

        let mut current_bias = 0.0;
        let mut current_strength = 0.0;

        for (i, &close) in closes.iter().enumerate() {
            let scale = close.abs().max(1.0);

            if self.swing_highs[i].is_finite() {
                last_high_2 = last_high_1;
                last_high_1 = Some((i, self.swing_highs[i]));
                is_above_last_high = close > self.swing_highs[i];
            }

            if self.swing_lows[i].is_finite() {
                last_low_2 = last_low_1;
                last_low_1 = Some((i, self.swing_lows[i]));
                is_below_last_low = close < self.swing_lows[i];
            }

            if let (Some((_, h0)), Some((idx_h1, h1)), Some((_, l0)), Some((idx_l1, l1))) =
                (last_high_2, last_high_1, last_low_2, last_low_1)
            {
                let hh = h1 > h0;
                let lh = h1 < h0;
                let hl = l1 > l0;
                let ll = l1 < l0;

                let hh_mag = if h0.abs() > 1e-12 {
                    ((h1 - h0) / h0.abs()).min(0.1) * 10.0
                } else {
                    0.0
                };
                let lh_mag = if h0.abs() > 1e-12 {
                    ((h0 - h1) / h0.abs()).min(0.1) * 10.0
                } else {
                    0.0
                };
                let hl_mag = if l0.abs() > 1e-12 {
                    ((l1 - l0) / l0.abs()).min(0.1) * 10.0
                } else {
                    0.0
                };
                let ll_mag = if l0.abs() > 1e-12 {
                    ((l0 - l1) / l0.abs()).min(0.1) * 10.0
                } else {
                    0.0
                };

                let mut raw = 0.0f64;

                if hh {
                    raw += 0.5 + hh_mag;
                }
                if hl {
                    raw += 0.5 + hl_mag;
                }
                if lh {
                    raw -= 0.5 + lh_mag;
                }
                if ll {
                    raw -= 0.5 + ll_mag;
                }

                let bias = (raw / 2.0).clamp(-1.0, 1.0);

                let bars_since_h1 = (i - idx_h1) as f64;
                let bars_since_l1 = (i - idx_l1) as f64;
                let avg_bars = (bars_since_h1 + bars_since_l1) / 2.0;
                let time_factor = (1.0 - (avg_bars / 250.0)).clamp(0.2, 1.0);

                let swing_move = ((h1 - h0).abs() + (l1 - l0).abs()) / scale;
                let amplitude = (swing_move * 20.0).tanh().clamp(0.0, 1.0);

                current_bias = bias * time_factor;
                current_strength = (bias.abs() * amplitude * time_factor).clamp(0.0, 1.0);
            } else {
                current_strength *= 0.995;
            }

            self.structure[i] = current_bias;
            self.structure_strength[i] = current_strength;

            if let Some((_, h1)) = last_high_1 {
                let broke_high = close > h1;

                if broke_high && !is_above_last_high {
                    let strength = (((close - h1) / scale) * 25.0).tanh().abs().clamp(0.0, 1.0);
                    if current_bias >= 0.0 {
                        self.bos[i] = strength;
                    } else {
                        self.choch[i] = strength;
                    }
                }
                is_above_last_high = broke_high;
            }

            if let Some((_, l1)) = last_low_1 {
                let broke_low = close < l1;

                if broke_low && !is_below_last_low {
                    let strength = (((l1 - close) / scale) * 25.0).tanh().abs().clamp(0.0, 1.0);
                    if current_bias <= 0.0 {
                        self.bos[i] = -strength;
                    } else {
                        self.choch[i] = -strength;
                    }
                }
                is_below_last_low = broke_low;
            }
        }
    }

    fn is_computed(&self) -> bool {
        !self.bos.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
