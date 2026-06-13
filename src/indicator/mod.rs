use crate::engine::Context;
use std::any::Any;

#[allow(unused)]
pub mod atr;

#[allow(unused)]
pub mod boll;

#[allow(unused)]
pub mod ema;

#[allow(unused)]
pub mod er;

#[allow(unused)]
pub mod roc;

#[allow(unused)]
pub mod rsi;

#[allow(unused)]
pub mod rvol;

#[allow(unused)]
pub mod stoch;

#[allow(unused)]
pub mod swing;

/// A market indicator that may depend on OHLCV data or other indicators.
pub trait Indicator: 'static {
    /// The name of the indicator.
    fn name() -> String
    where
        Self: Sized;

    /// Computes the indicator.
    fn compute(&mut self, ctx: Context);

    /// Returns true if the indicator has been computed.
    fn is_computed(&self) -> bool;

    /// Reset the indicator data.
    fn reset(&mut self);

    /// Returns a reference to the indicator as an [Any].
    fn as_any(&self) -> &dyn Any;
}
