use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;
use std::{thread, io, fmt};

use binance::model::KlineSummary;
use serde::{Serialize, Deserialize};
use crate::patterns::*;
use crate::strategies::*;

pub type StrategyFunc = fn(Vec<MathKLine>, StrategyParams, Arc<Vec<Arc<dyn PatternParams>>>) -> Vec<Trade>;
pub type Strategy = (StrategyFunc, StrategyParams, Arc<Vec<Arc<dyn PatternParams>>>);

const TAXES_SPOT: f64 = 0.000;
const TAXES_FUTURES: f64 = 0.0002;


#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum StrategyName {
    None,
    W,
    M,
    BullReversal
}

impl fmt::Display for StrategyName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StrategyName::None => write!(f, "None"),
            StrategyName::W => write!(f, "W"),
            StrategyName::M => write!(f, "M"),
            StrategyName::BullReversal => write!(f, "Bull Reversal"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Status {
    NotOpened,
    NotTriggered,
    Running,
    Closed(TradeResult)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TradeResult {
    Win,
    Lost,
    Unknown
}

#[derive(Clone, Debug)]
pub struct Trade {
    pub entry_price: f64,
    pub sl: f64,
    pub tp: f64,
    pub status: Status,
    pub open_time: i64,
    pub close_time: i64,
    pub money: f64,
    pub benefits: f64,
    pub loss: f64,
    pub taxes: f64,
    pub lots: f64,
    pub closing_kline: Option<MathKLine>,
    pub opening_kline: MathKLine,
    pub strategy: StrategyName,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StrategyResult{
    pub name: StrategyName,
    pub strategy_params: StrategyParams,
    pub patterns_params: HashMap<String, String>,
    pub win_ratio: f32,
    pub lose_ratio: f32,
    pub unknown_ratio: f32,
    pub total_win: usize,
    pub total_lose: usize,
    pub total_closed: usize,
    pub total_unclosed: usize,
    pub rr_ratio: f32,
    pub rr_lisible: String,
    pub efficiency: f32,
    pub final_money: f64,
    pub money_evolution: Vec<f64>
}

pub struct Backtester {
    klines_data: Vec<MathKLine>,
    trades: Vec<Trade>,
    num_workers: usize,
    strategies: Vec<Strategy>,
    results: Vec<StrategyResult>,
    current_strategy_money_evolution: Vec<f64>
}

impl Backtester {
    pub fn new(klines_data: Vec<KlineSummary>, num_workers: usize) -> Self {
        Backtester {
            klines_data: Self::to_all_math_kline(klines_data),
            trades: Vec::new(),
            num_workers,
            strategies: Vec::new(),
            results: Vec::new(),
            current_strategy_money_evolution: Vec::new()
        }
    }

    pub fn start(&mut self) -> &mut Self{
        let mut i = 0;
        let size = self.strategies.len();
        let start = Instant::now();

        for mut strategy in self.strategies.clone() {
            self.create_trades_from_strategy(strategy.clone());
            self.resolve_trades(&mut strategy);
            self.generate_results(strategy);
            self.clean_trades();

            i += 1;
            let duration = start.elapsed();
            print!("\rTrades resolved : {}% -- Elapsed time : {}s -- Estimated total time : {}s", 100*i/size, duration.as_secs(), ((duration.as_secs_f64()/i as f64)*size as f64) as u64);
            io::stdout().flush().unwrap();
        }
        self
    }

    fn create_trades(&mut self) {
        println!("Trade creation process starts. {} klines data to process", self.klines_data.len());
        let mut i = 0;
        let size = self.strategies.len();

        
        let start = Instant::now();
        
        for strategy in self.strategies.clone() {
            self.create_trades_from_strategy(strategy);
            i+=1;
            let duration = start.elapsed();
            print!("\rAvancement : {}% -- Elapsed time : {}s -- Estimated total time : {}s", 100*i/size, duration.as_secs(), ((duration.as_secs_f64()/i as f64)*size as f64) as u64);
            io::stdout().flush().unwrap();
        }
        println!();
        println!("created {} trades", self.trades.len());
    }

    fn create_trades_from_strategy(&mut self, strategy: Strategy) {
        let chunk_size = (self.klines_data.len() + self.num_workers - 1) / self.num_workers;
        let results = (0..self.num_workers).map(|i| {
            let start = i * chunk_size;
            let num_elements = if i < self.num_workers - 1 {
                chunk_size
            } else {
                self.klines_data.len() - i * chunk_size
            };

            let strategy_params_clone = strategy.1;
            let patterns_params_clone = strategy.2.clone();

            let chunk = Vec::from(&self.klines_data[start..][..num_elements]);

            thread::spawn(move || {
                strategy.0(chunk, strategy_params_clone, patterns_params_clone)
            })
        }).collect::<Vec<_>>();

        for handle in results {
            let mut partial_results = handle.join().unwrap();
            self.trades.append(&mut partial_results);
        }
    }

    fn resolve_trades(&mut self, strategy: &mut Strategy) {
        for kline in &self.klines_data {
            self.trades.iter_mut().for_each(|trade| {
                if kline.close_time == trade.open_time && trade.status == Status::NotOpened {
                    trade.status = Status::Running;
                    // taker et maker 0.1% de frais
                    //Nombre de lots = (Capital de départ x % de capital risqué dans le trade x Ratio risque/récompense) / (Prix d'entrée - Prix de stop-loss)
                    let mut lots = (strategy.1.money * strategy.1.risk_per_trade * (strategy.1.sl_multiplier/strategy.1.tp_multiplier)) / (trade.entry_price - trade.sl);
                    let taxes_rate;

                    match strategy.1.market_type {
                        MarketType::Spot => {
                            taxes_rate = TAXES_SPOT;
                            if lots * trade.entry_price > strategy.1.money {
                                lots = strategy.1.money / trade.entry_price;
                            }
                        }
                        MarketType::Futures => {
                            taxes_rate = TAXES_FUTURES;
                        }
                    }

                    let taxes = lots * trade.entry_price * taxes_rate;
                    strategy.1.money -= taxes;
                    trade.money = strategy.1.money;
                    trade.lots = lots;
                    trade.taxes = taxes;

                    trade.benefits = trade.money * strategy.1.risk_per_trade * strategy.1.tp_multiplier;
                    trade.loss = trade.money * strategy.1.risk_per_trade * strategy.1.sl_multiplier;

                    trade.benefits = lots * trade.tp - lots * trade.entry_price;
                    trade.loss = lots * trade.entry_price - lots * trade.sl;

                    //let leverage = (((trade.money + trade.benefits) / trade.money) - 1.) / ((trade.tp / trade.entry_price) - 1.);
                    //println!("leverage == {} for trade {:#?}", leverage, trade);
                    if trade.entry_price <= kline.high && trade.entry_price >= kline.low && trade.status == Status::NotTriggered{
                        trade.status = Status::Running;
                        //((Cap/Cav)-1)/((Pv/Pa)-1)
                    }
                }

                if kline.close_time > trade.open_time && trade.status == Status::Running {
                    if Self::hit_price(trade.sl, kline) && Self::hit_price(trade.tp, kline) {
                        trade.status = Status::Closed(TradeResult::Unknown);
                    } else if Self::hit_price(trade.tp, kline) {
                        strategy.1.money += trade.benefits;
                        self.current_strategy_money_evolution.push(strategy.1.money);
                        trade.status = Status::Closed(TradeResult::Win);
                    } else if Self::hit_price(trade.sl, kline) {
                        strategy.1.money -= trade.loss;
                        self.current_strategy_money_evolution.push(strategy.1.money);
                        trade.status = Status::Closed(TradeResult::Lost);
                    }
                    if strategy.1.money <= 0. {
                        return;
                    }
                }
            });
        }
    }

    fn generate_results(&mut self, strategy: Strategy) {
        let name = strategy.1.name;
        let mut patterns_params = HashMap::new();

        for params in strategy.2.as_ref() {
            patterns_params.extend(params.get_params());
        }
        
        let total_win = self.trades.iter().filter(|&trade| trade.status == Status::Closed(TradeResult::Win)).count();
        let total_lose = self.trades.iter().filter(|&trade| trade.status == Status::Closed(TradeResult::Lost)).count();
        let total_unknown = self.trades.iter().filter(|&trade| trade.status == Status::Closed(TradeResult::Unknown)).count();
        let total_closed = self.trades.iter().filter(|&trade| matches!(trade.status, Status::Closed{..})).count();
        let total_unclosed = self.trades.len() - total_closed;


        let win_ratio = (total_win as f32*100./total_closed as f32 * 100.0).round() / 100.0;
        let lose_ratio = (total_lose as f32*100./total_closed as f32 * 100.0).round() / 100.0;
        let unknown_ratio = (total_unknown as f32*100./total_closed as f32 * 100.0).round() / 100.0;
        let needed_win_percentage = (((1./(1.+(strategy.1.tp_multiplier/strategy.1.sl_multiplier))*100.) * 100.0).round() / 100.0) as f32;
        let efficiency = (win_ratio/needed_win_percentage * 100.0).round() / 100.0;
        let final_money = strategy.1.money;

        self.results.push(StrategyResult { 
            name,
            strategy_params: strategy.1,
            patterns_params,
            win_ratio,
            lose_ratio,
            unknown_ratio,
            total_win,
            total_lose,
            total_closed,
            total_unclosed,
            rr_ratio: (needed_win_percentage*0.01* 100.0).round() / 100.0,
            rr_lisible: format!("{}:{}", (strategy.1.tp_multiplier * (1./strategy.1.sl_multiplier) * 100.0).round() / 100.0, strategy.1.sl_multiplier * (1./strategy.1.sl_multiplier)),
            efficiency,
            final_money,
            money_evolution: self.current_strategy_money_evolution.clone()
         });
    }

    fn clean_trades(&mut self) {
        self.trades.clear();
        self.current_strategy_money_evolution.clear();
    }

    pub fn add_strategy(&mut self, strategy: Strategy) -> &mut Self {
        self.strategies.push(strategy);
        self
    }

    pub fn add_strategies(&mut self, strategies:&mut Vec<Strategy> ) -> &mut Self {
        self.strategies.append(strategies);
        self
    }

    pub fn get_wr_ratio(&self) -> (f32, f32, f32, usize) {
        let total_closed = self.trades.iter().filter(|&trade| matches!(trade.status, Status::Closed{..})).count() as f32;
        let win = self.trades.iter().filter(|&trade| trade.status == Status::Closed(TradeResult::Win)).count() as f32*100./total_closed;
        let loss = self.trades.iter().filter(|&trade| trade.status == Status::Closed(TradeResult::Lost)).count() as f32*100./total_closed;
        let unknown = self.trades.iter().filter(|&trade| trade.status == Status::Closed(TradeResult::Unknown)).count() as f32*100./total_closed;
        (win, loss, unknown, total_closed as usize)
    }

    pub fn get_wr_ratio_with_strategy(&self, strategy: StrategyName) -> (f32, f32, f32, usize) {
        let total_closed = self.trades.iter().filter(|&trade| matches!(trade.status, Status::Closed{..}) && trade.strategy == strategy).count() as f32;
        let win = self.trades.iter().filter(|&trade| trade.status == Status::Closed(TradeResult::Win) && trade.strategy == strategy).count() as f32*100./total_closed;
        let loss = self.trades.iter().filter(|&trade| trade.status == Status::Closed(TradeResult::Lost) && trade.strategy == strategy).count() as f32*100./total_closed;
        let unknown = self.trades.iter().filter(|&trade| trade.status == Status::Closed(TradeResult::Unknown) && trade.strategy == strategy).count() as f32*100./total_closed;
        (win, loss, unknown, total_closed as usize)
    }

    pub fn get_num_closed(&self) -> usize {
        let result = self.trades.iter().filter(|&trade| matches!(trade.status, Status::Closed{..})).count();
        result
    }

    pub fn get_num_status(&self, trade_status: Status) -> usize {
        let result = self.trades.iter().filter(|&trade| trade.status == trade_status).count();
        result
    }

    pub fn get_results(&self) -> Vec<StrategyResult> {
        self.results.clone()
    }

    fn hit_price(price: f64, kline: &MathKLine) -> bool {
        price <= kline.high && price >= kline.low
    }

    fn to_math_kline(kline: &KlineSummary) -> MathKLine{
        MathKLine {
            open_time: kline.open_time,
            open: kline.open.parse::<f64>().unwrap(),
            high: kline.high.parse::<f64>().unwrap(),
            low: kline.low.parse::<f64>().unwrap(),
            close: kline.close.parse::<f64>().unwrap(),
            volume: kline.volume.clone(),
            close_time: kline.close_time,
            quote_asset_volume: kline.quote_asset_volume.clone(),
            number_of_trades: kline.number_of_trades,
            taker_buy_base_asset_volume: kline.taker_buy_base_asset_volume.clone(),
            taker_buy_quote_asset_volume: kline.taker_buy_quote_asset_volume.clone()
        }
    }

    fn to_all_math_kline(klines: Vec<KlineSummary>) -> Vec<MathKLine>{
        let mut result: Vec<MathKLine> = Vec::new();
        for kline in klines.iter() {
            result.push(Self::to_math_kline(kline));
        }
        result
    }
}