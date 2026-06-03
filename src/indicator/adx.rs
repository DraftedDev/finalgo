use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

/// # Average Directional Movement Index (ADX)
///
/// ## Purpose
/// - Detect whether the market is trending or choppy
/// - Measure trend persistence
/// - Provide directional pressure from +DI / -DI separation
///
/// ## Math
///
/// ```
/// up_move   = high[t] - high[t - 1]
/// down_move = low[t - 1] - low[t]
///
/// +DM = up_move   if up_move > down_move && up_move > 0, otherwise 0
/// -DM = down_move if down_move > up_move && down_move > 0, otherwise 0
///
/// TR = max(
///     high[t] - low[t],
///     |high[t] - close[t - 1]|,
///     |low[t] - close[t - 1]|
/// )
///
/// smoothed_t = smoothed_{t-1} - (smoothed_{t-1} / period) + current_value
/// +DI = 100 * smoothed(+DM) / smoothed(TR)
/// -DI = 100 * smoothed(-DM) / smoothed(TR)
///
/// ADX = 100 * |+DI - -DI| / (+DI + -DI)
/// ```
pub struct AvgDirMovIdx<const PERIOD: usize> {
    pub adx: Vec<f64>,
    pub plus_di: Vec<f64>,
    pub minus_di: Vec<f64>,
    computed: bool,
}

impl<const PERIOD: usize> AvgDirMovIdx<PERIOD> {
    pub fn new() -> Self {
        Self {
            adx: Vec::new(),
            plus_di: Vec::new(),
            minus_di: Vec::new(),
            computed: false,
        }
    }
}

impl<const PERIOD: usize> Indicator for AvgDirMovIdx<PERIOD> {
    fn name(&self) -> String {
        format!("adx-{}", PERIOD)
    }

    fn compute(&mut self, int: &Interface) {
        let data = int.raw();
        let highs = &data.highs;
        let lows = &data.lows;
        let closes = &data.closes;

        let len = closes.len();

        self.adx = vec![f64::NAN; len];
        self.plus_di = vec![f64::NAN; len];
        self.minus_di = vec![f64::NAN; len];
        self.computed = false;

        if len <= PERIOD || PERIOD == 0 {
            return;
        }

        let mut tr = vec![f64::NAN; len];
        let mut plus_dm = vec![f64::NAN; len];
        let mut minus_dm = vec![f64::NAN; len];

        tr[0] = highs[0] - lows[0];
        plus_dm[0] = 0.0;
        minus_dm[0] = 0.0;

        for i in 1..len {
            let up_move = highs[i] - highs[i - 1];
            let down_move = lows[i - 1] - lows[i];

            plus_dm[i] = if up_move > down_move && up_move > 0.0 {
                up_move
            } else {
                0.0
            };

            minus_dm[i] = if down_move > up_move && down_move > 0.0 {
                down_move
            } else {
                0.0
            };

            tr[i] = f64::max(
                highs[i] - lows[i],
                f64::max(
                    (highs[i] - closes[i - 1]).abs(),
                    (lows[i] - closes[i - 1]).abs(),
                ),
            );
        }

        // Wilder smoothing:
        // initial sums use the first PERIOD raw values after index 0
        let mut smoothed_tr = vec![f64::NAN; len];
        let mut smoothed_plus_dm = vec![f64::NAN; len];
        let mut smoothed_minus_dm = vec![f64::NAN; len];

        if len <= PERIOD {
            return;
        }

        let init_start = 1;
        let init_end = PERIOD; // inclusive range [1..=PERIOD]

        let sum_tr: f64 = tr[init_start..=init_end].iter().copied().sum();
        let sum_plus: f64 = plus_dm[init_start..=init_end].iter().copied().sum();
        let sum_minus: f64 = minus_dm[init_start..=init_end].iter().copied().sum();

        smoothed_tr[PERIOD] = sum_tr;
        smoothed_plus_dm[PERIOD] = sum_plus;
        smoothed_minus_dm[PERIOD] = sum_minus;

        let mut dx = vec![f64::NAN; len];

        if smoothed_tr[PERIOD].is_finite() && smoothed_tr[PERIOD] > 0.0 {
            self.plus_di[PERIOD] = 100.0 * smoothed_plus_dm[PERIOD] / smoothed_tr[PERIOD];
            self.minus_di[PERIOD] = 100.0 * smoothed_minus_dm[PERIOD] / smoothed_tr[PERIOD];

            let denom = self.plus_di[PERIOD] + self.minus_di[PERIOD];
            if denom > 0.0 {
                dx[PERIOD] = 100.0 * (self.plus_di[PERIOD] - self.minus_di[PERIOD]).abs() / denom;
            }
        }

        for i in (PERIOD + 1)..len {
            smoothed_tr[i] = smoothed_tr[i - 1] - (smoothed_tr[i - 1] / PERIOD as f64) + tr[i];
            smoothed_plus_dm[i] =
                smoothed_plus_dm[i - 1] - (smoothed_plus_dm[i - 1] / PERIOD as f64) + plus_dm[i];
            smoothed_minus_dm[i] =
                smoothed_minus_dm[i - 1] - (smoothed_minus_dm[i - 1] / PERIOD as f64) + minus_dm[i];

            if smoothed_tr[i].is_finite() && smoothed_tr[i] > 0.0 {
                self.plus_di[i] = 100.0 * smoothed_plus_dm[i] / smoothed_tr[i];
                self.minus_di[i] = 100.0 * smoothed_minus_dm[i] / smoothed_tr[i];

                let denom = self.plus_di[i] + self.minus_di[i];
                if denom > 0.0 {
                    dx[i] = 100.0 * (self.plus_di[i] - self.minus_di[i]).abs() / denom;
                }
            }
        }

        // ADX smoothing of DX
        // Canonical start index is 2*PERIOD - 1
        let adx_start = PERIOD.saturating_mul(2).saturating_sub(1);
        if len <= adx_start {
            self.computed = true;
            return;
        }

        let dx_window_start = PERIOD;
        let dx_window_end = adx_start; // inclusive

        let mut dx_sum = 0.0;
        let mut dx_count = 0usize;

        for i in dx_window_start..=dx_window_end {
            if dx[i].is_finite() {
                dx_sum += dx[i];
                dx_count += 1;
            }
        }

        if dx_count == 0 {
            self.computed = true;
            return;
        }

        self.adx[adx_start] = dx_sum / dx_count as f64;

        for i in (adx_start + 1)..len {
            if dx[i].is_finite() && self.adx[i - 1].is_finite() {
                self.adx[i] = ((self.adx[i - 1] * (PERIOD as f64 - 1.0)) + dx[i]) / PERIOD as f64;
            }
        }

        self.computed = true;
    }

    fn is_computed(&self) -> bool {
        self.computed
    }

    fn score(&self) -> Vec<ScoreRecord> {
        if !self.computed || self.adx.is_empty() {
            return vec![];
        }

        let i = self.adx.len() - 1;

        let adx = self.adx[i];
        let plus = self.plus_di[i];
        let minus = self.minus_di[i];

        if !adx.is_finite() || !plus.is_finite() || !minus.is_finite() {
            return vec![];
        }

        let mut out = Vec::new();

        let di_sum = plus + minus;

        if di_sum > 0.0 {
            let direction = ((plus - minus) / di_sum).clamp(-1.0, 1.0);

            out.push(ScoreRecord::new(
                ScoreType::Direction,
                direction,
                0.90, // strong directional relevance
                0.80,
            ));
        }

        let strength = (adx / 50.0).clamp(0.0, 1.0);

        out.push(ScoreRecord::new(ScoreType::Strength, strength, 0.95, 0.90));

        let quality = ((adx - 20.0) / 30.0).clamp(0.0, 1.0);
        let quality = quality * 2.0 - 1.0;

        out.push(ScoreRecord::new(ScoreType::Quality, quality, 0.75, 0.85));

        let volatility = ((adx - 25.0) / 25.0).clamp(0.0, 1.0);
        let volatility = volatility * 2.0 - 1.0;

        out.push(ScoreRecord::new(
            ScoreType::Volatility,
            volatility,
            0.35,
            0.60,
        ));

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
