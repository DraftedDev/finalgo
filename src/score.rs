use crate::utils::assert_range;
use std::fmt::{Display, Formatter};

/// Multidimensional market scoring system.
///
/// Score ranges:
///
/// - Direction:  [-1.0,  1.0]
/// - Quality:    [-1.0,  1.0]
/// - Strength:   [ 0.0,  1.0]
/// - Volatility: [-1.0,  1.0]
///
/// Semantics:
///
/// Direction:
/// - negative => bearish bias
/// - positive => bullish bias
///
/// Quality:
/// - negative => noisy / chaotic
/// - positive => structured / clean
///
/// Strength:
/// - 0.0 => weak movement
/// - 1.0 => powerful movement
///
/// Volatility:
/// - negative => compression
/// - positive => expansion
#[derive(Debug, Default)]
pub struct Score {
    direction: Vec<ScoreRecord>,
    quality: Vec<ScoreRecord>,
    strength: Vec<ScoreRecord>,
    volatility: Vec<ScoreRecord>,
}

impl Score {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute final aggregated result.
    pub fn compute(&self) -> ScoreResult {
        let (direction, direction_label) = self.compute_score_of(ScoreType::Direction);

        let (quality, quality_label) = self.compute_score_of(ScoreType::Quality);

        let (strength, strength_label) = self.compute_score_of(ScoreType::Strength);

        let (volatility, volatility_label) = self.compute_score_of(ScoreType::Volatility);

        let signal = Self::final_signal(direction, strength, quality, volatility);

        let final_score = match signal {
            v if v >= 0.25 => FinalScore::Long,
            v if v <= -0.25 => FinalScore::Short,
            _ => FinalScore::Neutral,
        };

        ScoreResult {
            direction,
            direction_label,

            quality,
            quality_label,

            strength,
            strength_label,

            volatility,
            volatility_label,

            signal,
            final_score,
        }
    }

    /// Add a score record.
    pub fn add(&mut self, ty: ScoreType, record: ScoreRecord) {
        match ty {
            ScoreType::Direction => {
                assert_range(record.value, -1.0, 1.0, "direction");

                self.direction.push(record);
            }

            ScoreType::Quality => {
                assert_range(record.value, -1.0, 1.0, "quality");

                self.quality.push(record);
            }

            ScoreType::Strength => {
                assert_range(record.value, 0.0, 1.0, "strength");

                self.strength.push(record);
            }

            ScoreType::Volatility => {
                assert_range(record.value, -1.0, 1.0, "volatility");

                self.volatility.push(record);
            }
        }
    }

    /// Compute weighted average score for one score category.
    fn compute_score_of(&self, ty: ScoreType) -> (f64, String) {
        let records = match ty {
            ScoreType::Direction => &self.direction,
            ScoreType::Quality => &self.quality,
            ScoreType::Strength => &self.strength,
            ScoreType::Volatility => &self.volatility,
        };

        if records.is_empty() {
            return (0.0, "No data".to_string());
        }

        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;

        for r in records {
            weighted_sum += r.value * r.weight;
            weight_sum += r.weight;
        }

        if weight_sum <= f64::EPSILON {
            return (0.0, "No data".to_string());
        }

        let value = weighted_sum / weight_sum;

        let value = match ty {
            ScoreType::Strength => value.clamp(0.0, 1.0),

            _ => value.clamp(-1.0, 1.0),
        };

        let label = match ty {
            ScoreType::Direction => Self::direction_label(value),

            ScoreType::Quality => Self::quality_label(value),

            ScoreType::Strength => Self::strength_label(value),

            ScoreType::Volatility => Self::volatility_label(value),
        };

        (value, label)
    }

    /// Final combined trading signal.
    ///
    /// Output range:
    /// [-1.0, 1.0]
    ///
    /// Notes:
    ///
    /// - Direction is primary driver.
    /// - Strength amplifies conviction.
    /// - Quality validates signal reliability.
    /// - Volatility dampens unstable regimes.
    fn final_signal(direction: f64, strength: f64, quality: f64, volatility: f64) -> f64 {
        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;

        // Directional core
        for (value, weight) in [
            (direction, 0.50),
            (quality, 0.30),
            (strength * direction.signum(), 0.20),
        ] {
            weighted_sum += value * weight;
            weight_sum += weight;
        }

        if weight_sum <= f64::EPSILON {
            return 0.0;
        }

        let base_signal = (weighted_sum / weight_sum).clamp(-1.0, 1.0);

        let volatility_factor = 1.0 - 0.35 * volatility.abs().clamp(0.0, 1.0);

        (base_signal * volatility_factor).clamp(-1.0, 1.0)
    }

    fn direction_label(v: f64) -> String {
        match v {
            v if v >= 0.75 => "Strong Bullish",
            v if v >= 0.25 => "Bullish",
            v if v > -0.25 => "Neutral",
            v if v > -0.75 => "Bearish",
            _ => "Strong Bearish",
        }
        .to_string()
    }

    fn quality_label(v: f64) -> String {
        match v {
            v if v >= 0.75 => "Excellent Structure",
            v if v >= 0.25 => "Good Structure",
            v if v > -0.25 => "Mixed",
            v if v > -0.75 => "Noisy",
            _ => "Chaotic",
        }
        .to_string()
    }

    fn strength_label(v: f64) -> String {
        match v {
            v if v >= 0.85 => "Extreme Momentum",
            v if v >= 0.60 => "High Momentum",
            v if v >= 0.30 => "Moderate Momentum",
            _ => "Weak Momentum",
        }
        .to_string()
    }

    fn volatility_label(v: f64) -> String {
        match v {
            v if v >= 0.75 => "Extreme Expansion",
            v if v >= 0.25 => "Expansion",
            v if v > -0.25 => "Neutral Regime",
            v if v > -0.75 => "Compression",
            _ => "Deep Compression",
        }
        .to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreType {
    Direction,
    Quality,
    Strength,
    Volatility,
}

#[derive(Debug, Clone, Copy)]
pub struct ScoreRecord {
    value: f64,
    weight: f64,
}

impl ScoreRecord {
    pub fn new(value: f64, weight: f64) -> Self {
        assert!(value.is_finite(), "score value must be finite");

        assert!(weight.is_finite(), "score weight must be finite");

        assert!(
            (0.0..=1.0).contains(&weight),
            "weight must be within [0.0, 1.0]"
        );

        Self { value, weight }
    }
}

#[derive(Debug, Clone)]
pub struct ScoreResult {
    /// [-1.0, 1.0]
    pub direction: f64,
    pub direction_label: String,

    /// [-1.0, 1.0]
    pub quality: f64,
    pub quality_label: String,

    /// [0.0, 1.0]
    pub strength: f64,
    pub strength_label: String,

    /// [-1.0, 1.0]
    pub volatility: f64,
    pub volatility_label: String,

    pub signal: f64,
    pub final_score: FinalScore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalScore {
    Long,
    Short,
    Neutral,
}

impl Display for FinalScore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FinalScore::Long => write!(f, "Long"),
            FinalScore::Short => write!(f, "Short"),
            FinalScore::Neutral => write!(f, "Neutral"),
        }
    }
}
