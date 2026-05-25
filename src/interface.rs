use crate::indicator::Indicator;
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
use crate::utils;
use crate::utils::{naive_to_offset, round_to_two_decimals};
use std::any::TypeId;
use std::collections::HashMap;
use std::time::Duration;
use yahoo_finance_api::YahooConnectorBuilder;

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

        self.run_order.push(id);
        self.indicators.insert(id, Box::new(indicator));
    }

    pub fn run(&mut self) -> ScoreResult {
        tracing::info!("Building {} indicators...", self.indicators.len());

        for id in &self.run_order {
            let mut indicator = self.indicators.remove(id).unwrap();

            tracing::info!("Computing indicator '{}'...", indicator.name());
            indicator.compute(self);

            for (ty, score) in indicator.score() {
                self.score.add(ty, score);
            }

            self.indicators.insert(*id, indicator);
        }

        tracing::info!("[#############################################]");
        let score = self.score.compute();

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

        score
    }
}

pub struct StockData {
    pub highs: Vec<f64>,
    pub lows: Vec<f64>,
    pub opens: Vec<f64>,
    pub closes: Vec<f64>,
    pub volumes: Vec<f64>,
}

impl StockData {
    pub async fn fetch_range(end: String, lookback: usize, ticker: String) -> Self {
        let yahoo = YahooConnectorBuilder::new()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to build yahoo connector");

        let end = utils::parse_naive_date(&end);
        let start = utils::subtract_naive_date(end, lookback);

        let start = naive_to_offset(start);
        let end = naive_to_offset(end);

        let mut response = yahoo
            .get_quote_history(&ticker, start, end)
            .await
            .expect("Failed to request quotes")
            .quotes()
            .expect("Failed to get quotes");

        // Ensure deterministic ordering
        response.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Self {
            highs: response.iter().map(|q| q.high).collect(),
            lows: response.iter().map(|q| q.low).collect(),
            opens: response.iter().map(|q| q.open).collect(),
            closes: response.iter().map(|q| q.close).collect(),
            volumes: response.iter().map(|q| q.volume as f64).collect(),
        }
    }

    pub async fn fetch_single(end: String, ticker: String) -> Self {
        let mut data = Self::fetch_range(end, 5, ticker).await;

        // Keep ONLY last candle deterministically
        let idx = data.closes.len().saturating_sub(1);

        data.highs = vec![data.highs[idx]];
        data.lows = vec![data.lows[idx]];
        data.opens = vec![data.opens[idx]];
        data.closes = vec![data.closes[idx]];
        data.volumes = vec![data.volumes[idx]];

        data
    }
}
