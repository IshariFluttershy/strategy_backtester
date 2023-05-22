use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, channel};
use std::time::Instant;
use std::{thread, io, fmt};

use binance::model::KlineSummary;
use chrono::Duration;
use serde::{Serialize, Deserialize};
use crate::patterns::*;
use crate::strategies::*;

pub type StrategyFunc = fn(&Vec<MathKLine>, &Sender<f32>, StrategyParams, Arc<Vec<Arc<dyn PatternParams>>>) -> Vec<Trade>;
pub type Strategy = (StrategyFunc, StrategyParams, Arc<Vec<Arc<dyn PatternParams>>>);

const TAXES_SPOT: f64 = 0.000;
const TAXES_FUTURES: f64 = 0.0002;
const MAX_LEVERAGE: f64 = 10.;



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
    klines_data: Arc<Vec<MathKLine>>,
    trades: Vec<Trade>,
    strategies: Vec<Strategy>,
    results: Vec<StrategyResult>,
    current_strategy_money_evolution: Vec<f64>,
    progression_tracker: Sender<(f32, usize)>,
    id: usize,
}

impl Backtester {
    pub fn new(klines_data: Arc<Vec<MathKLine>>, progression_tracker: Sender<(f32, usize)>, id: usize) -> Self {
        Backtester {
            klines_data,
            trades: Vec::new(),
            strategies: Vec::new(),
            results: Vec::new(),
            current_strategy_money_evolution: Vec::new(),
            progression_tracker,
            id
        }
    }

    pub fn start(&mut self) -> &mut Self{
        let size = self.strategies.len();
        let start = Instant::now();
        let (tx, rx) = channel::<f32>();

        let progression_tracker_clone = self.progression_tracker.clone();
        let id = self.id;
        let total = self.strategies.len();
        let current: Arc<Mutex<f32>> = Arc::new(Mutex::new(1.));
        let current_clone: Arc<Mutex<f32>> = current.clone();
        thread::spawn(move || loop {
            while let Ok(progression) = rx.recv() {
                let current_num = *current.lock().unwrap();
                let total_sent = ((progression + ((current_num-1.)*100.))/(total * 100) as f32) * 100.;
                //println!("progression == {} ---- total sent == {} ---- current == {}", progression, total_sent, current_num);
                progression_tracker_clone.send((total_sent, id));
            }
        });

        for (i, strategy) in self.strategies.clone().iter_mut().enumerate() {
            self.create_trades_from_strategy(strategy.clone(), &tx);
            self.resolve_trades(strategy, &tx);
            self.generate_results(strategy);
            self.clean_trades();
            
            let new_current = *current_clone.lock().unwrap() + 1.;
            *current_clone.lock().unwrap() = new_current;
        }
        self.progression_tracker.send((100., self.id));
        self
    }

    fn create_trades_from_strategy(&mut self, strategy: Strategy, progression_tracker: &Sender<f32>) {
        self.trades = strategy.0(&self.klines_data, &progression_tracker, strategy.1, strategy.2);
    }

    fn resolve_trades(&mut self, strategy: &mut Strategy, progression_tracker: &Sender<f32>) {
        let mut last_sent = 0;
        let mut start = 0;
        for (i, kline) in self.klines_data.iter().enumerate() {
            let mut j = start;
            while j < self.trades.len() {
                let mut trade = &mut self.trades[j];
                if let Status::Closed(_) = trade.status {
                    if j == start + 1 {
                        start = j;
                    }
                    j += 1;
                    continue;
                } else if trade.open_time > kline.close_time {
                    break;
                }
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
                                lots = (strategy.1.money * MAX_LEVERAGE) / trade.entry_price;
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
                    if kline.low <= trade.sl && kline.high >= trade.tp {
                        trade.status = Status::Closed(TradeResult::Unknown);
                    } else if kline.low <= trade.sl {
                        trade.status = Status::Closed(TradeResult::Lost);
                        strategy.1.money -= trade.loss;
                        self.current_strategy_money_evolution.push(strategy.1.money);
                    } else if kline.high >= trade.tp {
                        trade.status = Status::Closed(TradeResult::Win);
                        strategy.1.money += trade.benefits;
                        self.current_strategy_money_evolution.push(strategy.1.money);
                    }
                    if strategy.1.money <= 0. {
                        return;
                    }
                }
                j += 1;
            };
            if last_sent + 1000 < i {
                let to_send = (i as f32/self.klines_data.len() as f32*50.) + 50.;
                progression_tracker.send(to_send);
                last_sent = i;
            }
        }
    }

    fn generate_results(&mut self, strategy: &Strategy) {
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

    // fonction qui porte très mal son nom
    /*fn hit_price(price: f64, kline: &MathKLine) -> bool {
        price <= kline.high && price >= kline.low
    }*/

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

    pub fn to_all_math_kline(klines: Vec<KlineSummary>) -> Vec<MathKLine>{
        let mut result: Vec<MathKLine> = Vec::new();
        for kline in klines.iter() {
            result.push(Self::to_math_kline(kline));
        }
        result
    }
}