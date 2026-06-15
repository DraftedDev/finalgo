use crate::data::StockData;
use crate::indicator::Indicator;
use crate::indicator::atr::AvgTrueRange;
use crate::indicator::boll::BollingerBands;
use crate::indicator::ema::ExpMovAvg;
use crate::indicator::er::EfficiencyRatio;
use crate::indicator::roc::RateOfChange;
use crate::indicator::rsi::RelStrengthIdx;
use crate::indicator::rvol::RelativeVolume;
use crate::indicator::swing::SwingStructure;
use crate::regime::Regime;
use crate::score::Score;
use crate::score::final_score::FinalScore;
use crate::score::participation::ParticipationScore;
use crate::score::quality::QualityScore;
use crate::score::strength::StrengthScore;
use crate::score::trend::TrendScore;
use crate::score::volatility::VolatilityScore;
use crate::utils::{FastMap, ValueMap};

/// Builds the engine with complete set of indicators and scores.
pub fn build() -> Engine {
    let mut engine = Engine::new();

    engine.add_indicator(AvgTrueRange::<14>::new());
    engine.add_indicator(BollingerBands::<20, 2>::new());
    engine.add_indicator(BollingerBands::<30, 2>::new());
    engine.add_indicator(ExpMovAvg::<100>::new());
    engine.add_indicator(ExpMovAvg::<600>::new());
    engine.add_indicator(RateOfChange::<10>::new());
    engine.add_indicator(EfficiencyRatio::<10, 3>::new());
    engine.add_indicator(SwingStructure::<5, 10>::new());
    engine.add_indicator(SwingStructure::<10, 10>::new());
    engine.add_indicator(RelativeVolume::<20>::new());
    engine.add_indicator(RelStrengthIdx::<14>::new());

    engine.add_score(TrendScore::new());
    engine.add_score(StrengthScore::new());
    engine.add_score(QualityScore::new());
    engine.add_score(VolatilityScore::new());
    engine.add_score(ParticipationScore::new());
    engine.add_score(FinalScore::new());

    engine
}

/// The engine behind the algorithm
pub struct Engine {
    regime: Option<Regime>,

    indicators: FastMap<String, Box<dyn Indicator>>,
    run_indicators: Vec<String>,

    scores: FastMap<String, Box<dyn Score>>,
    run_scores: Vec<String>,
}

impl Engine {
    /// Creates a new engine instance. Not recommended to use directly.
    ///
    /// See [build] for a more convenient way to create an engine.
    pub fn new() -> Self {
        Self {
            regime: None,
            indicators: FastMap::with_capacity_and_hasher(16, Default::default()),
            run_indicators: Vec::with_capacity(16),

            scores: FastMap::with_capacity_and_hasher(16, Default::default()),
            run_scores: Vec::with_capacity(16),
        }
    }

    /// Returns the context for the engine with the given [StockData].
    pub fn context<'a>(&'a self, data: &'a StockData) -> Context<'a> {
        Context { engine: self, data }
    }

    /// Adds an indicator to the engine.
    ///
    /// Panics if the indicator is already registered.
    pub fn add_indicator<I: Indicator>(&mut self, indicator: I) {
        let name = I::name();

        if self.indicators.contains_key(&name) {
            panic!("Indicator already registered");
        }

        self.indicators.insert(name.clone(), Box::new(indicator));
        self.run_indicators.push(name);
    }

    /// Adds a score to the engine.
    ///
    /// Panics if the score is already registered.
    pub fn add_score<S: Score>(&mut self, score: S) {
        let name = S::name();

        if self.scores.contains_key(&name) {
            panic!("Score already registered");
        }

        self.scores.insert(name.clone(), Box::new(score));
        self.run_scores.push(name);
    }

    /// Executes the algorithm with the given [StockData].
    ///
    /// Panics if the data is empty.
    #[tracing::instrument(skip_all)]
    pub fn compute(&mut self, traces: bool, data: &StockData) -> ValueMap {
        assert!(!data.highs.is_empty(), "Highs must not be empty");
        assert!(!data.lows.is_empty(), "Lows must not be empty");
        assert!(!data.opens.is_empty(), "Opens must not be empty");
        assert!(!data.closes.is_empty(), "Closes must not be empty");
        assert!(!data.volumes.is_empty(), "Volumes must not be empty");

        if traces {
            tracing::info!("Building {} indicators...", self.indicators.len());
        }

        for name in &self.run_indicators {
            let mut indicator = self.indicators.remove(name).unwrap();

            if traces {
                tracing::info!("Computing indicator '{name}'...");
            }

            if indicator.is_computed() {
                panic!("Indicator {name} already computed!");
            }

            indicator.compute(self.context(data));

            self.indicators.insert(name.clone(), indicator);
        }

        if traces {
            tracing::info!("Computing market regime...");
        }

        self.regime = Some(Regime::compute(self.context(data)));

        if traces {
            tracing::info!("Building {} scores...", self.scores.len());
        }

        let mut result = ValueMap::new();

        for name in &self.run_scores {
            let mut score = self.scores.remove(name).unwrap();

            if traces {
                tracing::info!("Computing score '{name}'...");
            }

            if score.is_computed() {
                panic!("Score {name} already computed!");
            }

            let new_result = score.compute(self.context(data));

            result.merge(new_result);
            self.scores.insert(name.clone(), score);
        }

        result
    }

    /// Resets the internal indicator and score states.
    pub fn reset(&mut self) {
        for ind in self.indicators.values_mut() {
            ind.reset();
        }

        for score in self.scores.values_mut() {
            score.reset();
        }
    }
}

/// The context behind an engine combined with the [StockData].
pub struct Context<'a> {
    engine: &'a Engine,
    data: &'a StockData,
}

impl<'a> Context<'a> {
    /// Returns the [StockData] associated with the context.
    pub fn data(&self) -> &'a StockData {
        self.data
    }

    /// Returns the market regime.
    pub fn regime(&self) -> Regime {
        self.engine.regime.clone().expect("Regime not computed")
    }

    /// Returns the indicator with the given type.
    ///
    /// Panics if the indicator is not found or has not been computed yet.
    pub fn indicator<I: Indicator>(&self) -> &I {
        let name = I::name();

        let ind = self
            .engine
            .indicators
            .get(&name)
            .unwrap_or_else(|| panic!("Indicator {name} not found"))
            .as_any()
            .downcast_ref::<I>()
            .unwrap();

        assert!(ind.is_computed(), "Indicator {name} not computed");

        ind
    }

    /// Returns the score with the given type.
    ///
    /// Panics if the score is not found or has not been computed yet.
    pub fn score<S: Score>(&self) -> &S {
        let name = S::name();

        let score = self
            .engine
            .scores
            .get(&name)
            .unwrap_or_else(|| panic!("Score {name} not found"))
            .as_any()
            .downcast_ref::<S>()
            .unwrap();

        assert!(score.is_computed(), "Score {name} not computed");

        score
    }
}
