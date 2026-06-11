use crate::engine::Context;
use crate::utils::ValueMap;
use std::any::Any;

pub mod participation;
pub mod quality;
pub mod strength;
pub mod trend;
pub mod volatility;

/// A score based on indicators and optionally other scores if needed.
pub trait Score: 'static {
    /// The name of the score.
    fn name(&self) -> String;

    /// Computes the score.
    fn compute(&mut self, ctx: Context) -> ValueMap;

    /// Returns true if the score has been computed.
    fn is_computed(&self) -> bool;

    /// Resets the internal score data.
    fn reset(&mut self);

    /// Returns a reference to the score as an [Any].
    fn as_any(&self) -> &dyn Any;
}
