use crate::indicator::Indicator;
use crate::interface::Interface;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

pub struct SwingStructure<const WINDOW: usize> {
    swing_highs: Vec<Option<f64>>,
    swing_lows: Vec<Option<f64>>,
}

impl<const WINDOW: usize> SwingStructure<WINDOW> {
    pub fn new() -> Self {
        Self {
            swing_highs: Vec::new(),
            swing_lows: Vec::new(),
        }
    }
}

impl<const WINDOW: usize> Indicator for SwingStructure<WINDOW> {
    fn name(&self) -> String {
        format!("swing_structure-{}", WINDOW)
    }

    fn compute(&mut self, int: &Interface) {
        let data = int.raw();
        let highs = &data.highs;
        let lows = &data.lows;

        let len = highs.len();

        self.swing_highs = vec![None; len];
        self.swing_lows = vec![None; len];

        for i in WINDOW..(len - WINDOW) {
            let mut is_swing_high = true;
            let mut is_swing_low = true;

            let h = highs[i];
            let l = lows[i];

            for j in (i - WINDOW)..(i + WINDOW + 1) {
                if highs[j] > h {
                    is_swing_high = false;
                }
                if lows[j] < l {
                    is_swing_low = false;
                }
            }

            if is_swing_high {
                self.swing_highs[i] = Some(h);
            }

            if is_swing_low {
                self.swing_lows[i] = Some(l);
            }
        }
    }

    fn is_computed(&self) -> bool {
        !self.swing_highs.is_empty()
    }

    fn score(&self, int: &Interface) -> Vec<ScoreRecord> {
        let mut out = Vec::new();

        let len = self.swing_highs.len();

        let mut last_highs: Vec<f64> = Vec::new();
        let mut last_lows: Vec<f64> = Vec::new();

        let mut last_swing_high = None;
        let mut last_swing_low = None;

        for i in 0..len {
            let high = self.swing_highs[i];
            let low = self.swing_lows[i];

            // update swing structure
            if let Some(h) = high {
                if let Some(_) = last_swing_high {
                    last_highs.push(h);
                }
                last_swing_high = Some(h);
            }

            if let Some(l) = low {
                if let Some(_) = last_swing_low {
                    last_lows.push(l);
                }
                last_swing_low = Some(l);
            }

            // need at least structure
            if last_highs.len() < 2 || last_lows.len() < 2 {
                continue;
            }

            let hh = last_highs[last_highs.len() - 1] > last_highs[last_highs.len() - 2];
            let hl = last_lows[last_lows.len() - 1] > last_lows[last_lows.len() - 2];

            let lh = last_highs[last_highs.len() - 1] < last_highs[last_highs.len() - 2];
            let ll = last_lows[last_lows.len() - 1] < last_lows[last_lows.len() - 2];

            // direction
            let direction = if hh && hl {
                1.0
            } else if lh && ll {
                -1.0
            } else {
                0.0
            };

            // structure quality (clean HH/HL or LL/LH)
            let structure_strength = if (hh && hl) || (lh && ll) { 1.0 } else { 0.3 };

            // breakout detection
            let breakout = if let Some(last_h) = last_swing_high {
                if let Some(last_l) = last_swing_low {
                    let current_price = int.raw().highs[i];

                    if current_price > last_h {
                        1.0
                    } else if current_price < last_l {
                        -1.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            } else {
                0.0
            };

            let confidence = structure_strength;

            let weight = 0.9;

            out.push(ScoreRecord::new(
                ScoreType::Direction,
                direction,
                weight,
                confidence,
            ));

            out.push(ScoreRecord::new(
                ScoreType::Strength,
                structure_strength,
                weight,
                confidence,
            ));

            out.push(ScoreRecord::new(
                ScoreType::Quality,
                breakout, // reuse quality channel for BOS signal
                0.8,
                confidence,
            ));
        }

        out
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
