use crate::engine::Context;
use crate::indicator::Indicator;
use std::any::Any;

/// # SwingStructure Indicator
///
/// Captures market structure via swing highs/lows, trend bias,
/// and structural events (BOS / CHoCH).
pub struct SwingStructure<const LEFT: usize, const RIGHT: usize> {
    /// Confirmed swing highs.
    ///
    /// Stores pivot highs that are higher than surrounding candles
    /// within a LEFT/RIGHT window.
    ///
    /// Non-swing points are NaN.
    pub swing_highs: Vec<f64>,

    /// Confirmed swing lows.
    ///
    /// Stores pivot lows that are lower than surrounding candles
    /// within a LEFT/RIGHT window.
    ///
    /// Non-swing points are NaN.
    pub swing_lows: Vec<f64>,

    /// Structural bias signal.
    ///
    /// Represents trend direction derived from swing progression:
    ///
    /// - positive -> bullish structure (HH + HL behavior)
    /// - negative -> bearish structure (LH + LL behavior)
    /// - 0 → transitional / unclear structure
    pub structure: Vec<f64>,

    /// Strength of structural trend.
    ///
    /// Combines:
    /// - magnitude of swing progression
    /// - consistency of structure direction
    ///
    /// Range:
    /// - 0.0 → no structural conviction
    /// - 1.0 → strong directional structure
    pub structure_strength: Vec<f64>,

    /// Break of Structure (BOS) events.
    ///
    /// Indicates continuation of existing trend structure:
    ///
    /// - positive value -> bullish BOS
    /// - negative value -> bearish BOS
    /// - 0.0 → no BOS event
    ///
    /// Strength reflects breakout magnitude beyond swing level.
    pub bos: Vec<f64>,

    /// Change of Character (CHoCH) events.
    ///
    /// Indicates potential trend reversal:
    ///
    /// - positive value -> bullish reversal shift
    /// - negative value -> bearish reversal shift
    /// - 0.0 -> no structural change event
    ///
    /// Strength reflects breakout magnitude beyond swing level.
    pub choch: Vec<f64>,
}

impl<const LEFT: usize, const RIGHT: usize> SwingStructure<LEFT, RIGHT> {
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
    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-12
    }

    #[inline]
    fn is_unique_max(window: &[f64], candidate: f64) -> bool {
        if !candidate.is_finite() {
            return false;
        }

        let mut count = 0;
        for &v in window {
            if !v.is_finite() {
                return false;
            }
            if v > candidate {
                return false;
            }
            if Self::approx_eq(v, candidate) {
                count += 1;
            }
        }

        count == 1
    }

    #[inline]
    fn is_unique_min(window: &[f64], candidate: f64) -> bool {
        if !candidate.is_finite() {
            return false;
        }

        let mut count = 0;
        for &v in window {
            if !v.is_finite() {
                return false;
            }
            if v < candidate {
                return false;
            }
            if Self::approx_eq(v, candidate) {
                count += 1;
            }
        }

        count == 1
    }
}

impl<const LEFT: usize, const RIGHT: usize> Indicator for SwingStructure<LEFT, RIGHT> {
    fn name(&self) -> String {
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

        self.swing_highs = vec![f64::NAN; len];
        self.swing_lows = vec![f64::NAN; len];
        self.structure = vec![0.0; len];
        self.structure_strength = vec![0.0; len];
        self.bos = vec![0.0; len];
        self.choch = vec![0.0; len];

        // Confirm pivots
        for i in LEFT..(len - RIGHT) {
            let high_window = &highs[i - LEFT..=i + RIGHT];
            let low_window = &lows[i - LEFT..=i + RIGHT];

            let high = highs[i];
            let low = lows[i];

            if Self::is_unique_max(high_window, high) {
                self.swing_highs[i] = high;
            }

            if Self::is_unique_min(low_window, low) {
                self.swing_lows[i] = low;
            }
        }

        // Build structure
        let mut last_high_1: Option<(usize, f64)> = None;
        let mut last_high_2: Option<(usize, f64)> = None;
        let mut last_low_1: Option<(usize, f64)> = None;
        let mut last_low_2: Option<(usize, f64)> = None;

        for i in 0..len {
            let close = closes[i];

            if self.swing_highs[i].is_finite() {
                last_high_2 = last_high_1;
                last_high_1 = Some((i, self.swing_highs[i]));
            }

            if self.swing_lows[i].is_finite() {
                last_low_2 = last_low_1;
                last_low_1 = Some((i, self.swing_lows[i]));
            }

            let (Some((_, h0)), Some((_, h1)), Some((_, l0)), Some((_, l1))) =
                (last_high_2, last_high_1, last_low_2, last_low_1)
            else {
                continue;
            };

            let hh = h1 > h0;
            let lh = h1 < h0;
            let hl = l1 > l0;
            let ll = l1 < l0;

            let mut raw = 0.0f64;

            if hh {
                raw += 1.0;
            }
            if hl {
                raw += 1.0;
            }
            if lh {
                raw -= 1.0;
            }
            if ll {
                raw -= 1.0;
            }

            let bias = (raw / 2.0).clamp(-1.0, 1.0);

            let scale = close.abs().max(1.0);
            let swing_move = ((h1 - h0).abs() + (l1 - l0).abs()) / scale;
            let amplitude = (swing_move * 20.0).tanh().clamp(0.0, 1.0);

            self.structure[i] = bias;
            self.structure_strength[i] = (bias.abs() * amplitude).clamp(0.0, 1.0);

            // EVENT-BASED BOS / CHoCH (FIXED)
            if close.is_finite() {
                let prev_close = if i > 0 { closes[i - 1] } else { close };

                // bullish break
                if prev_close <= h1 && close > h1 {
                    let strength = (((close - h1) / scale) * 25.0).tanh().abs().clamp(0.0, 1.0);

                    if bias >= 0.0 {
                        self.bos[i] = strength;
                    } else {
                        self.choch[i] = strength;
                    }
                }

                // bearish break
                if prev_close >= l1 && close < l1 {
                    let strength = (((l1 - close) / scale) * 25.0).tanh().abs().clamp(0.0, 1.0);

                    if bias <= 0.0 {
                        self.bos[i] = -strength;
                    } else {
                        self.choch[i] = -strength;
                    }
                }
            }
        }
    }

    fn is_computed(&self) -> bool {
        !self.bos.is_empty()
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
