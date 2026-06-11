use crate::data::StockData;
use crate::indicator::Indicator;
use crate::indicator::atr::AvgTrueRange;
use crate::indicator::boll::BollingerBands;
use crate::indicator::ema::ExpMovAvg;
use crate::indicator::er::EfficiencyRatio;
use crate::indicator::roc::RateOfChange;
use crate::indicator::rvol::RelativeVolume;
use crate::indicator::swing::SwingStructure;
use crate::regime::Regime;
use crate::score::Score;
use crate::score::participation::ParticipationScore;
use crate::score::quality::QualityScore;
use crate::score::strength::StrengthScore;
use crate::score::trend::TrendScore;
use crate::score::volatility::VolatilityScore;
use crate::utils::{FastMap, ValueMap};
use std::any::TypeId;

pub fn build() -> Engine {
    let mut engine = Engine::new();

    engine.add_indicator(ExpMovAvg::<20>::new());
    engine.add_indicator(ExpMovAvg::<600>::new());
    engine.add_indicator(RateOfChange::<10>::new());
    engine.add_indicator(EfficiencyRatio::<10, 3>::new());
    engine.add_indicator(SwingStructure::<10, 10>::new());
    engine.add_indicator(SwingStructure::<5, 10>::new());
    engine.add_indicator(SwingStructure::<5, 5>::new());
    engine.add_indicator(AvgTrueRange::<14>::new());
    engine.add_indicator(BollingerBands::<20, 2>::new());
    engine.add_indicator(BollingerBands::<30, 2>::new());
    engine.add_indicator(RelativeVolume::<20>::new());

    engine.add_score(TrendScore::new());
    engine.add_score(StrengthScore::new());
    engine.add_score(QualityScore::new());
    engine.add_score(VolatilityScore::new());
    engine.add_score(ParticipationScore::new());

    engine
}

pub struct Engine {
    regime: Option<Regime>,

    indicators: FastMap<TypeId, Box<dyn Indicator>>,
    run_indicators: Vec<TypeId>,

    scores: FastMap<TypeId, Box<dyn Score>>,
    run_scores: Vec<TypeId>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            regime: None,
            indicators: FastMap::with_capacity_and_hasher(16, Default::default()),
            run_indicators: Vec::with_capacity(16),

            scores: FastMap::with_capacity_and_hasher(16, Default::default()),
            run_scores: Vec::with_capacity(16),
        }
    }

    pub fn context<'a>(&'a self, data: &'a StockData) -> Context<'a> {
        Context { engine: self, data }
    }

    pub fn add_indicator<I: Indicator>(&mut self, indicator: I) {
        let id = TypeId::of::<I>();

        if self.indicators.contains_key(&id) {
            panic!("Indicator already registered");
        }

        self.indicators.insert(id, Box::new(indicator));
        self.run_indicators.push(id);
    }

    pub fn add_score<S: Score>(&mut self, score: S) {
        let id = TypeId::of::<S>();

        if self.scores.contains_key(&id) {
            panic!("Score already registered");
        }

        self.scores.insert(id, Box::new(score));
        self.run_scores.push(id);
    }

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

        for id in &self.run_indicators {
            let mut indicator = self.indicators.remove(id).unwrap();
            let name = indicator.name();

            if traces {
                tracing::info!("Computing indicator '{name}'...");
            }

            if indicator.is_computed() {
                panic!("Indicator {name} already computed!");
            }

            indicator.compute(self.context(data));

            self.indicators.insert(*id, indicator);
        }

        if traces {
            tracing::info!("Computing market regime...");
        }

        self.regime = Some(Regime::compute(self.context(data)));

        if traces {
            tracing::info!("Building {} scores...", self.scores.len());
        }

        let mut result = ValueMap::new();

        for id in &self.run_scores {
            let mut score = self.scores.remove(id).unwrap();
            let name = score.name();

            if traces {
                tracing::info!("Computing score '{name}'...");
            }

            if score.is_computed() {
                panic!("Score {name} already computed!");
            }

            let new_result = score.compute(self.context(data));

            result.merge(new_result);
            self.scores.insert(*id, score);
        }

        result
    }

    pub fn reset(&mut self) {
        for ind in self.indicators.values_mut() {
            ind.reset();
        }

        for score in self.scores.values_mut() {
            score.reset();
        }
    }
}

pub struct Context<'a> {
    engine: &'a Engine,
    data: &'a StockData,
}

impl<'a> Context<'a> {
    pub fn data(&self) -> &'a StockData {
        self.data
    }

    pub fn regime(&self) -> Regime {
        self.engine.regime.clone().expect("Regime not computed")
    }

    pub fn indicator<I: Indicator>(&self) -> &I {
        let ind = self
            .engine
            .indicators
            .get(&TypeId::of::<I>())
            .expect("Indicator not found")
            .as_any()
            .downcast_ref::<I>()
            .unwrap();

        assert!(ind.is_computed(), "Indicator {} not computed", ind.name());

        ind
    }

    pub fn score<S: Score>(&self) -> &S {
        let score = self
            .engine
            .scores
            .get(&TypeId::of::<S>())
            .expect("Score not found")
            .as_any()
            .downcast_ref::<S>()
            .unwrap();

        assert!(score.is_computed(), "Score {} not computed", score.name());

        score
    }
}
