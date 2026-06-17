use crate::engine::Context;
use std::any::Any;

/// Contains the [final_score::FinalScore] score.
pub mod final_score;

/// Contains the [participation::ParticipationScore] score.
pub mod participation;

/// Contains the [quality::QualityScore] score.
pub mod quality;

/// Contains the [strength::StrengthScore] score.
pub mod strength;

/// Contains the [trend::TrendScore] score.
pub mod trend;

/// Contains the [volatility::VolatilityScore] score.
pub mod volatility;

/// A score based on indicators and optionally other scores if needed.
pub trait Score: 'static {
    /// The name of the score.
    fn name() -> String
    where
        Self: Sized;

    /// Computes the score.
    fn compute(&mut self, ctx: Context);

    /// Returns true if the score has been computed.
    fn is_computed(&self) -> bool;

    /// Returns a reference to the score as an [Any].
    fn as_any(&self) -> &dyn Any;
}
