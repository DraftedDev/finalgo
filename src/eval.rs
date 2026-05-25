use crate::interface;
use crate::interface::{Interface, StockData};
use crate::score::{FinalScore, ScoreResult};

/// Evaluates prediction quality against future realized candles.
///
/// Architecture:
/// - prediction = indicator/score system output
/// - target = synthesized future market behavior
/// - loss = bounded [0, 1]
///
/// Lower loss = better.
pub struct Evaluator {
    data: Vec<(Interface, StockData)>,
}

impl Evaluator {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn add(&mut self, predict: StockData, target: StockData) {
        self.data.push((interface::build(predict), target));
    }

    pub fn eval(&mut self) -> Vec<ScoreLoss> {
        let mut losses = Vec::with_capacity(self.data.len());

        for (int, target_data) in &mut self.data {
            let predict = int.run();
            let target = Self::build_target_result(target_data);

            losses.push(ScoreLoss::new(predict, target));
        }

        losses
    }

    fn build_target_result(target: &StockData) -> ScoreResult {
        assert!(
            !target.opens.is_empty(),
            "Target stock data must contain at least 1 candle"
        );

        let len = target.opens.len();

        let open = target.opens[0];
        let close = target.closes[len - 1];

        let high = target
            .highs
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        let low = target.lows.iter().copied().fold(f64::INFINITY, f64::min);

        let raw_return = if open.abs() > 1e-12 {
            (close - open) / open
        } else {
            0.0
        };

        let direction = (raw_return * 12.0).tanh().clamp(-1.0, 1.0);

        let strength = (raw_return.abs() * 12.0).tanh().clamp(0.0, 1.0);

        let range = (high - low).max(0.0);
        let normalized_range = if open.abs() > 1e-12 {
            range / open
        } else {
            0.0
        };

        let volatility = ((normalized_range - 0.02) * 40.0).tanh().clamp(-1.0, 1.0);

        let body = (close - open).abs();
        let body_ratio = if range > 1e-12 { body / range } else { 0.0 };
        let quality = (body_ratio * 2.0 - 1.0).clamp(-1.0, 1.0);

        let signal = direction * strength;

        let final_score = if signal > 0.25 {
            FinalScore::Long
        } else if signal < -0.25 {
            FinalScore::Short
        } else {
            FinalScore::Neutral
        };

        ScoreResult {
            direction,
            direction_label: String::new(),

            quality,
            quality_label: String::new(),

            strength,
            strength_label: String::new(),

            volatility,
            volatility_label: String::new(),

            signal,
            final_score,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScoreLoss {
    /// Signed directional alignment loss.
    ///
    /// [0, 1]
    pub direction: f64,

    /// Structural similarity loss.
    ///
    /// [0, 1]
    pub quality: f64,

    /// Magnitude mismatch loss.
    ///
    /// [0, 1]
    pub strength: f64,

    /// Regime mismatch loss.
    ///
    /// [0, 1]
    pub volatility: f64,

    /// Final classification mismatch.
    ///
    /// [0, 1]
    pub signal: f64,
}

impl ScoreLoss {
    pub fn new(predict: ScoreResult, target: ScoreResult) -> Self {
        let direction = Self::signed_loss(predict.direction, target.direction);

        let quality = Self::signed_loss(predict.quality, target.quality);

        let strength = Self::unsigned_loss(predict.strength, target.strength);

        let volatility = Self::signed_loss(predict.volatility, target.volatility);

        let signal = Self::signal_loss(predict.final_score, target.final_score);

        Self {
            direction,
            quality,
            strength,
            volatility,
            signal,
        }
    }

    /// Signed alignment loss.
    ///
    /// Input:
    /// [-1, 1]
    ///
    /// Output:
    /// [0, 1]
    ///
    /// 0.0 = perfect alignment
    ///
    /// 1.0 = complete opposition
    fn signed_loss(pred: f64, target: f64) -> f64 {
        if !pred.is_finite() || !target.is_finite() {
            return 1.0;
        }

        let pred = pred.clamp(-1.0, 1.0);
        let target = target.clamp(-1.0, 1.0);

        ((1.0 - pred * target) / 2.0).clamp(0.0, 1.0)
    }

    /// Unsigned magnitude loss.
    ///
    /// Input:
    /// [0, 1]
    ///
    /// Output:
    /// [0, 1]
    fn unsigned_loss(pred: f64, target: f64) -> f64 {
        if !pred.is_finite() || !target.is_finite() {
            return 1.0;
        }

        let pred = pred.clamp(0.0, 1.0);
        let target = target.clamp(0.0, 1.0);

        (pred - target).abs().clamp(0.0, 1.0)
    }

    /// Final directional classification loss.
    ///
    /// 0.0 = exact match
    /// 0.5 = neutral mismatch
    /// 1.0 = opposite side
    fn signal_loss(pred: FinalScore, target: FinalScore) -> f64 {
        match (pred, target) {
            (FinalScore::Long, FinalScore::Long) => 0.0,
            (FinalScore::Short, FinalScore::Short) => 0.0,
            (FinalScore::Neutral, FinalScore::Neutral) => 0.0,

            (FinalScore::Neutral, _) => 0.5,
            (_, FinalScore::Neutral) => 0.5,

            _ => 1.0,
        }
    }

    pub fn total(&self) -> f64 {
        (self.direction * 0.35
            + self.strength * 0.20
            + self.quality * 0.15
            + self.volatility * 0.10
            + self.signal * 0.20)
            .clamp(0.0, 1.0)
    }

    pub fn aggregate(losses: &[ScoreLoss]) -> Self {
        assert!(!losses.is_empty(), "Losses must not be empty");

        let n = losses.len() as f64;

        let mut direction = 0.0;
        let mut quality = 0.0;
        let mut strength = 0.0;
        let mut volatility = 0.0;
        let mut signal = 0.0;

        for loss in losses {
            direction += loss.direction;
            quality += loss.quality;
            strength += loss.strength;
            volatility += loss.volatility;
            signal += loss.signal;
        }

        Self {
            direction: direction / n,
            quality: quality / n,
            strength: strength / n,
            volatility: volatility / n,
            signal: signal / n,
        }
    }
}
