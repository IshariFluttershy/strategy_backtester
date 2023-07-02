use std::sync::Arc;

use crate::backtest::*;
use crate::patterns::*;
use crate::strategies;
use crate::strategies::*;

#[derive(Clone, Copy)]
pub struct ParamMultiplier<T> {
    pub min: T,
    pub max: T,
    pub step: T,
}

pub fn create_w_and_m_pattern_strategies(
    start_money: f64,
    tp: ParamMultiplier<f64>,
    sl: ParamMultiplier<f64>,
    klines_repetitions: ParamMultiplier<usize>,
    klines_range: ParamMultiplier<usize>,
    risk: ParamMultiplier<f64>,
    market_type: MarketType
) -> Vec<Strategy> {
    let mut strategies: Vec<Strategy> = Vec::new();
    let mut i = tp.min;
    while i <= tp.max {
        let mut j = sl.min;
        while j <= sl.max {
            let mut k = klines_repetitions.min;
            while k <= klines_repetitions.max {
                let mut l = klines_range.min;
                while l <= klines_range.max {
                    let mut m = risk.min;
                    while m <= risk.max {
                        let pattern_params_w: Vec<Arc<dyn PatternParams>> =
                            vec![Arc::new(WPatternParams {
                                klines_repetitions: k,
                                klines_range: l,
                                name: PatternName::W,
                            })];

                        let pattern_params_m: Vec<Arc<dyn PatternParams>> =
                            vec![Arc::new(MPatternParams {
                                klines_repetitions: k,
                                klines_range: l,
                                name: PatternName::M,
                            })];

                        strategies.push((
                            strategies::create_wpattern_trades,
                            StrategyParams {
                                tp_multiplier: i,
                                sl_multiplier: j,
                                risk_per_trade: m * 0.01,
                                money: start_money,
                                name: StrategyName::W,
                                market_type
                            },
                            Arc::new(pattern_params_w),
                        ));

                        strategies.push((
                            strategies::create_mpattern_trades,
                            StrategyParams {
                                tp_multiplier: i,
                                sl_multiplier: j,
                                risk_per_trade: m * 0.01,
                                money: start_money,
                                name: StrategyName::M,
                                market_type
                            },
                            Arc::new(pattern_params_m),
                        ));
                        m += risk.step;
                    }
                    l += klines_range.step;
                }
                k += klines_repetitions.step;
            }
            j += sl.step;
        }
        i += tp.step;
    }

    strategies
}

pub fn create_reversal_pattern_strategies(
    start_money: f64,
    tp: ParamMultiplier<f64>,
    sl: ParamMultiplier<f64>,
    trend_size: ParamMultiplier<usize>,
    counter_trend_size: ParamMultiplier<usize>,
    risk: ParamMultiplier<f64>,
    market_type: MarketType
) -> Vec<Strategy> {
    let mut strategies: Vec<Strategy> = Vec::new();
    let mut i = tp.min;
    while i <= tp.max {
        let mut j = sl.min;
        while j <= sl.max {
            let mut k: usize = trend_size.min;
            while k <= trend_size.max {
                let mut l = counter_trend_size.min;
                while l <= counter_trend_size.max {
                    let mut m = risk.min;
                    while m <= risk.max {
                        let reversal_pattern_params: Vec<Arc<dyn PatternParams>> =
                            vec![Arc::new(ReversalPatternParams {
                                trend_size: k,
                                counter_trend_size: l,
                                name: PatternName::BullReversal,
                            })];

                        strategies.push((
                            strategies::create_bull_reversal_trades,
                            StrategyParams {
                                tp_multiplier: i,
                                sl_multiplier: j,
                                risk_per_trade: m * 0.01,
                                money: start_money,
                                name: StrategyName::BullReversal,
                                market_type
                            },
                            Arc::new(reversal_pattern_params),
                        ));
                        m += risk.step;
                    }
                    l += counter_trend_size.step;
                }
                k += trend_size.step;
            }
            j += sl.step;
        }
        i += tp.step;
    }
    strategies
}