use crate::engine::Context;
use std::any::Any;

/// Contains the [atr::AvgTrueRange] indicator.
pub mod atr;

/// Contains the [boll::BollingerBands] indicator.
pub mod boll;

/// Contains the [ema::ExpMovAvg] indicator.
pub mod ema;

/// Contains the [er::EfficiencyRatio] indicator.
pub mod er;

/// Contains the [roc::RateOfChange] indicator.
pub mod roc;

/// Contains the [rsi::RelStrengthIdx] indicator.
pub mod rsi;

/// Contains the [rvol::RelativeVolume] indicator.
pub mod rvol;

/// Contains the [stoch::Stochastic] indicator.
#[allow(unused)]
pub mod stoch;

/// Contains the [swing::SwingStructure] indicator.
pub mod swing;

/// Contains the [exits::DynamicExits] indicator.
pub mod exits;

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

    /// Returns a reference to the indicator as an [Any].
    fn as_any(&self) -> &dyn Any;
}
