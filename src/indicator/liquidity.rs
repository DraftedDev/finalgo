use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

/// # Liquidity Sweep / Stop Hunt Detector
///
/// ## Purpose
/// - Detect stop-loss hunting above/below key swing levels
/// - Identify false breakouts (sweeps with rejection)
/// - Capture reversal-driven liquidity grabs
/// - Improve directional precision in breakout regimes
///
/// ## Math
///
/// ### Swing levels (rolling liquidity reference)
///
/// ```text
/// swing_high_t = max(high_{t-N ... t-1})
/// swing_low_t  = min(low_{t-N ... t-1})
///
/// sweep_up_t = high_t > swing_high_t
/// sweep_down_t = low_t < swing_low_t
/// extension_up_t = (high_t - swing_high_t) / swing_high_t
///
/// extension_down_t = (swing_low_t - low_t) / swing_low_t
/// rejection_up_t = (swing_high_t - close_t) / swing_high_t
/// rejection_down_t = (close_t - swing_low_t) / swing_low_t
///
/// signal_t = -1
/// strength_t = clamp(extension_up_t, 0, 1)
///
/// signal_t = +1
/// strength_t = clamp(extension_down_t, 0, 1)
///
/// signal_t = 0
/// strength_t = 0
///
/// quality_t = clamp(2 * rejection_t - 0.5, -1, 1)
///
/// rejection_t = rejection_up_t OR rejection_down_t
/// ```
pub struct LiquiditySweep<const LOOKBACK: usize> {
    sweep_signal: Vec<f64>,
    sweep_strength: Vec<f64>,
    rejection_strength: Vec<f64>,
}

impl<const LOOKBACK: usize> LiquiditySweep<LOOKBACK> {
    pub fn new() -> Self {
        Self {
            sweep_signal: Vec::new(),
            sweep_strength: Vec::new(),
            rejection_strength: Vec::new(),
        }
    }

    #[inline]
    fn safe_scale(reference: f64) -> f64 {
        if reference.is_finite() && reference.abs() > 1e-12 {
            reference.abs()
        } else {
            1.0
        }
    }
}

impl<const LOOKBACK: usize> Indicator for LiquiditySweep<LOOKBACK> {
    fn name(&self) -> String {
        format!("liquidity_sweep-{}", LOOKBACK)
    }

    fn compute(&mut self, int: &Interface) {
        let data = int.raw();
        let highs = &data.highs;
        let lows = &data.lows;
        let closes = &data.closes;

        let len = highs.len();

        self.sweep_signal = vec![0.0; len];
        self.sweep_strength = vec![0.0; len];
        self.rejection_strength = vec![0.0; len];

        if len <= LOOKBACK {
            return;
        }

        for i in LOOKBACK..len {
            let window_high = highs[i - LOOKBACK..i]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);

            let window_low = lows[i - LOOKBACK..i]
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);

            let high = highs[i];
            let low = lows[i];
            let close = closes[i];

            if !high.is_finite() || !low.is_finite() || !close.is_finite() {
                continue;
            }

            let range = window_high - window_low;
            if !range.is_finite() || range <= 1e-12 {
                continue;
            }

            // Normalize all distances by recent range scale,
            // not by price level alone.
            let scale = Self::safe_scale((window_high + window_low + close) / 3.0);

            // Above range high -> bearish sweep pressure
            let up_break = ((high - window_high) / scale).max(0.0);
            let up_reject = ((window_high - close) / scale).max(0.0);

            // Below range low -> bullish sweep pressure
            let down_break = ((window_low - low) / scale).max(0.0);
            let down_reject = ((close - window_low) / scale).max(0.0);

            // Continuous event strength.
            // A sweep is stronger when:
            // - it extends beyond a known level
            // - and closes back away from that extreme
            let bearish_pressure = up_break * (0.5 + up_reject).clamp(0.5, 1.5);
            let bullish_pressure = down_break * (0.5 + down_reject).clamp(0.5, 1.5);

            let signal = (bullish_pressure - bearish_pressure).clamp(-1.0, 1.0);
            let strength = (bullish_pressure + bearish_pressure).clamp(0.0, 1.0);

            let rejection = up_reject.max(down_reject).clamp(0.0, 1.0);

            self.sweep_signal[i] = signal;
            self.sweep_strength[i] = strength;
            self.rejection_strength[i] = rejection;
        }
    }

    fn is_computed(&self) -> bool {
        !self.sweep_signal.is_empty()
    }

    fn score(&self, _: &Interface) -> Vec<ScoreRecord> {
        let mut out = Vec::new();

        let len = self.sweep_signal.len();
        if len == 0 {
            return out;
        }

        for i in 0..len {
            let signal = self.sweep_signal[i];
            let strength = self.sweep_strength[i];
            let rejection = self.rejection_strength[i];

            if !signal.is_finite() || !strength.is_finite() || !rejection.is_finite() {
                continue;
            }

            if signal.abs() <= 1e-12 && strength <= 1e-12 {
                continue;
            }

            // Direction:
            // - bearish sweep above highs => negative
            // - bullish sweep below lows  => positive
            let direction = signal.clamp(-1.0, 1.0);

            // Strength:
            // larger extension = stronger event
            let final_strength = strength.clamp(0.0, 1.0);

            // Quality:
            // rejection should be high when the sweep is meaningful
            // and low when the sweep is weak / indecisive.
            let quality = (rejection * 2.0 - 0.5).clamp(-1.0, 1.0);

            // Confidence:
            // use both extension and rejection.
            let confidence = (0.45 * final_strength + 0.55 * rejection).clamp(0.0, 1.0);

            // Recency weighting:
            // recent sweeps should matter more than old ones.
            let recency = if len > 1 {
                i as f64 / (len - 1) as f64
            } else {
                1.0
            };
            let weight = (0.25 + 0.75 * recency).clamp(0.0, 1.0);

            out.push(ScoreRecord::new(
                ScoreType::Direction,
                direction,
                weight,
                confidence,
            ));

            out.push(ScoreRecord::new(
                ScoreType::Strength,
                final_strength,
                weight,
                confidence,
            ));

            out.push(ScoreRecord::new(
                ScoreType::Quality,
                quality,
                weight,
                confidence,
            ));
        }

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
