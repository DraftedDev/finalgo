use crate::interface::Interface;
use crate::score::ScoreRecord;
use std::any::Any;

#[allow(unused)]
pub mod adx;

#[allow(unused)]
pub mod atr;

#[allow(unused)]
pub mod bol_width;

#[allow(unused)]
pub mod donchian;

#[allow(unused)]
pub mod ema;

#[allow(unused)]
pub mod er;

#[allow(unused)]
pub mod rel_vol;

#[allow(unused)]
pub mod roc;

#[allow(unused)]
pub mod rsi;

#[allow(unused)]
pub mod stochastic;

#[allow(unused)]
pub mod swing;

#[allow(unused)]
pub mod liquidity;

pub trait Indicator: 'static {
    fn name(&self) -> String;

    fn compute(&mut self, int: &Interface);
    fn is_computed(&self) -> bool;

    fn score(&self, int: &Interface) -> Vec<ScoreRecord>;

    fn as_any(&self) -> &dyn Any;
}
