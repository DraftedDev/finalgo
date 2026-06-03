use crate::data::StockData;
use crate::indicator::Indicator;
use crate::indicator::adx::AvgDirMovIdx;
use crate::indicator::atr::AvgTrueRange;
use crate::indicator::bol_width::BollingerWidth;
use crate::indicator::donchian::DonchianPosition;
use crate::indicator::ema::ExpMovAvg;
use crate::indicator::er::EfficiencyRatio;
use crate::indicator::rel_vol::RelativeVolume;
use crate::indicator::roc::RateOfChange;
use crate::indicator::rsi::RelStrengthIdx;
use crate::indicator::stochastic::Stochastic;
use crate::score::{Score, ScoreResult};
use crate::utils::round_to_two_decimals;
use std::any::TypeId;
use std::collections::HashMap;

pub fn build(data: StockData) -> Interface {
    let mut interface = Interface::new(data);

    interface.add_indicator(AvgTrueRange::<14>::new());
    interface.add_indicator(BollingerWidth::<20>::new());
    interface.add_indicator(EfficiencyRatio::<10>::new());
    interface.add_indicator(RateOfChange::<10>::new());
    interface.add_indicator(ExpMovAvg::<20>::new());
    interface.add_indicator(RelStrengthIdx::<14>::new());
    interface.add_indicator(Stochastic::<14, 3>::new());
    interface.add_indicator(DonchianPosition::<20>::new());
    interface.add_indicator(RelativeVolume::<20>::new());
    interface.add_indicator(AvgDirMovIdx::<14>::new());

    interface
}

pub struct Interface {
    data: StockData,
    indicators: HashMap<TypeId, Box<dyn Indicator>>,
    run_order: Vec<TypeId>,
    score: Score,
}

impl Interface {
    fn new(data: StockData) -> Self {
        Self {
            data,
            indicators: HashMap::new(),
            run_order: Vec::new(),
            score: Score::new(),
        }
    }

    pub fn raw(&self) -> &StockData {
        &self.data
    }

    pub fn indicator<I: Indicator>(&self) -> &I {
        let ind = self
            .indicators
            .get(&TypeId::of::<I>())
            .expect("Indicator not found")
            .as_any()
            .downcast_ref::<I>()
            .unwrap();

        assert!(
            ind.is_computed(),
            "Indicator {} not initialized",
            ind.name()
        );

        ind
    }

    pub fn add_indicator<I: Indicator>(&mut self, indicator: I) {
        let id = TypeId::of::<I>();

        if self.indicators.contains_key(&id) {
            panic!("Indicator already registered");
        }

        self.run_order.push(id);
        self.indicators.insert(id, Box::new(indicator));
    }

    pub fn run(&mut self, traces: bool) -> ScoreResult {
        if traces {
            tracing::info!("Building {} indicators...", self.indicators.len());
        }

        for id in &self.run_order {
            let mut indicator = self.indicators.remove(id).unwrap();

            if traces {
                tracing::info!("Computing indicator '{}'...", indicator.name());
            }
            indicator.compute(self);

            for score in indicator.score() {
                self.score.add(score);
            }

            self.indicators.insert(*id, indicator);
        }

        let score = self.score.compute();

        if traces {
            tracing::info!("[#############################################]");

            tracing::info!(
                "DIRECTION   || {:+} ({})",
                round_to_two_decimals(score.direction),
                score.direction_label
            );

            tracing::info!(
                "QUALITY     || {:+} ({})",
                round_to_two_decimals(score.quality),
                score.quality_label
            );

            tracing::info!(
                "STRENGTH    || {:+} ({})",
                round_to_two_decimals(score.strength),
                score.strength_label
            );

            tracing::info!(
                "VOLATILITY  || {:+} ({})",
                round_to_two_decimals(score.volatility),
                score.volatility_label
            );

            tracing::info!(
                "FINAL SCORE || {:+} ({})",
                round_to_two_decimals(score.signal),
                score.final_score
            );

            tracing::info!("[#############################################]");
        }

        score
    }
}
