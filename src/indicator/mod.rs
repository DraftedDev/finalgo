use crate::interface::Interface;
use crate::score::{ScoreRecord, ScoreType};
use std::any::Any;

pub mod atr;
pub mod bol_width;
pub mod donchian;
pub mod ema;
pub mod er;
pub mod rel_vol;
pub mod roc;
pub mod rsi;
pub mod stochastic;

pub trait Indicator: 'static {
    fn name(&self) -> String;

    fn compute(&mut self, int: &Interface);
    fn is_computed(&self) -> bool;

    fn score(&self) -> Vec<(ScoreType, ScoreRecord)>;

    fn as_any(&self) -> &dyn Any;
}
