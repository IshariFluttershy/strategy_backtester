use std::sync::Arc;
use std::sync::mpsc::Sender;

use serde::Deserialize;
use serde::Serialize;

use crate::backtest::*;
use crate::patterns::*;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum MarketType {
    Spot,
    Futures
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct StrategyParams {
    pub tp_multiplier: f64,
    pub sl_multiplier: f64,
    pub risk_per_trade: f64,
    #[serde(skip_serializing)]
    pub money: f64,
    pub name: StrategyName,
    pub market_type: MarketType,
}

pub fn create_wpattern_trades(
    chunk: &Vec<MathKLine>,
    progression_tracker: &Sender<f32>,
    strategy_params: StrategyParams,
    patterns_params: Arc<Vec<Arc<dyn PatternParams>>>,
) -> Vec<Trade> {
    let mut result_vec = Vec::new();
    let mut j = 0;
    let mut last_sent = 0;
    while j < chunk.len() {
        if let Some(wpattern_params) = patterns_params[0].downcast_ref::<WPatternParams>() {
            if let Some(result) = find_w_pattern(&chunk[j..], *wpattern_params) {
                    j += result.end_index;
                    result_vec.push(Trade {
                        entry_price: result.neckline_price,
                        sl: result.lower_price - (result.neckline_price - result.lower_price) * (strategy_params.sl_multiplier - 1.),
                        tp: result.neckline_price
                            + (result.neckline_price - result.lower_price)
                                * strategy_params.tp_multiplier,
                        open_time: result.end_time,
                        opening_kline: chunk[j].clone(),
                        money: 0.,
                        benefits: 0.,
                        loss: 0.,
                        taxes: 0.,
                        lots: 0.,
                        close_time: 0,
                        closing_kline: None,
                        status: Status::NotOpened,
                        strategy: strategy_params.name,
                    });
            } else {
                j += 1;
            }
            if last_sent + 1000 < j {
                progression_tracker.send(j as f32/chunk.len() as f32*50.);
                last_sent = j;
            }
        }
    }
    result_vec
}

pub fn create_mpattern_trades(
    chunk: &Vec<MathKLine>,
    progression_tracker: &Sender<f32>,
    strategy_params: StrategyParams,
    patterns_params: Arc<Vec<Arc<dyn PatternParams>>>,
) -> Vec<Trade> {
    let mut result_vec = Vec::new();
    let mut j = 0;
    let mut last_sent = 0;
    while j < chunk.len() {
        if let Some(mpattern_params) = patterns_params[0].downcast_ref::<MPatternParams>() {
            if let Some(result) = find_m_pattern(&chunk[j..], *mpattern_params) {
                    j += result.end_index;
                    result_vec.push(Trade {
                        entry_price: result.neckline_price,
                        sl: result.higher_price - ((result.neckline_price - result.higher_price) * (strategy_params.sl_multiplier - 1.)),
                        tp: result.neckline_price + ((result.neckline_price - result.higher_price) * strategy_params.tp_multiplier),
                        open_time: result.end_time,
                        opening_kline: chunk[j].clone(),
                        money: 0.,
                        benefits: 0.,
                        loss: 0.,
                        taxes: 0.,
                        lots: 0.,
                        close_time: 0,
                        closing_kline: None,
                        status: Status::NotOpened,
                        strategy: strategy_params.name,
                    });
            } else {
                j += 1;
            }
            if last_sent + 1000 < j {
                progression_tracker.send(j as f32/chunk.len() as f32*50.);
                last_sent = j;
            }
        }
    }
    result_vec
}

pub fn create_bull_reversal_trades(
    chunk: &Vec<MathKLine>,
    progression_tracker: &Sender<f32>,
    strategy_params: StrategyParams,
    patterns_params: Arc<Vec<Arc<dyn PatternParams>>>,
) -> Vec<Trade> {
    let mut result_vec = Vec::new();
    let mut j = 0;
    let mut last_sent = 0;
    while j < chunk.len() {
        if let Some(reversal_pattern_params) = patterns_params[0].downcast_ref::<ReversalPatternParams>() {
            if let Some(result) = find_bull_reversal(&chunk[j..], *reversal_pattern_params) {
                    j += result.end_index;
                    result_vec.push(Trade {
                        entry_price: result.end_price,
                        sl: result.peak_price * strategy_params.sl_multiplier,
                        tp: result.end_price
                            + ((result.end_price - result.peak_price)
                                * strategy_params.tp_multiplier),
                        open_time: result.end_time,
                        opening_kline: chunk[j].clone(),
                        money: 0.,
                        benefits: 0.,
                        loss: 0.,
                        taxes: 0.,
                        lots: 0.,
                        close_time: 0,
                        closing_kline: None,
                        status: Status::NotOpened,
                        strategy: strategy_params.name,
                    });
            } else {
                j += 1;
            }
            if last_sent + 1000 < j {
                progression_tracker.send(j as f32/chunk.len() as f32*50.);
                last_sent = j;
            }
        }
    }
    result_vec
}