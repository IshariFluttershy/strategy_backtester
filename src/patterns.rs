use std::collections::HashMap;
use std::fmt;

use downcast_rs::DowncastSync;
use downcast_rs::impl_downcast;
static mut _KLINE_TIME: i64 = 0;

pub trait PatternParams: DowncastSync { fn get_params(&self) -> HashMap<String, String>; }
impl_downcast!(PatternParams);
impl PatternParams for WPatternParams {  
    fn get_params(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert(String::from("klines_repetitions"), self.klines_repetitions.to_string());
        map.insert(String::from("klines_range"), self.klines_range.to_string());
        map.insert(String::from("name"), self.name.to_string());
        map
    }
}
impl PatternParams for MPatternParams {  
    fn get_params(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert(String::from("klines_repetitions"), self.klines_repetitions.to_string());
        map.insert(String::from("klines_range"), self.klines_range.to_string());
        map.insert(String::from("name"), self.name.to_string());
        map
    }
}
impl PatternParams for ReversalPatternParams {  
    fn get_params(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert(String::from("trend_size"), self.trend_size.to_string());
        map.insert(String::from("counter_trend_size"), self.counter_trend_size.to_string());
        map.insert(String::from("name"), self.name.to_string());
        map
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PatternName {
    None,
    W,
    M,
    BullReversal
}

impl fmt::Display for PatternName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PatternName::None => write!(f, "None"),
            PatternName::W => write!(f, "W"),
            PatternName::M => write!(f, "M"),
            PatternName::BullReversal => write!(f, "Bull Reversal"),
        }
    }
}

#[derive(Debug)]
pub struct WPattern {
    pub start_index: usize,
    pub start_time: i64,
    pub end_index: usize,
    pub end_time: i64,
    pub lower_price: f64,
    pub neckline_price: f64
}

#[derive(Debug)]
pub struct MPattern {
    pub start_index: usize,
    pub start_time: i64,
    pub end_index: usize,
    pub end_time: i64,
    pub higher_price: f64,
    pub neckline_price: f64
}

#[derive(Debug)]
pub struct ReversalPattern {
    pub start_index: usize,
    pub start_time: i64,
    pub end_index: usize,
    pub end_time: i64,
    pub peak_price: f64,
    pub end_price: f64
}

#[derive(Clone, PartialEq, Debug)]
pub struct MathKLine {
    pub open_time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: String,
    pub close_time: i64,
    pub quote_asset_volume: String,
    pub number_of_trades: i64,
    pub taker_buy_base_asset_volume: String,
    pub taker_buy_quote_asset_volume: String,
}

#[derive(Clone, PartialEq, Debug)]
struct TestParams {
    price: Option<f64>,
    kline: Option<MathKLine>
}

struct TestFunction {
    function: fn (MathKLine, Option<TestParams>) -> bool,
    params: Option<TestParams>,
}

#[derive(Copy, Clone, Debug)]
pub struct WPatternParams {
    pub klines_repetitions: usize,
    pub klines_range: usize,
    pub name: PatternName
}

#[derive(Copy, Clone, Debug)]
pub struct MPatternParams {
    pub klines_repetitions: usize,
    pub klines_range: usize,
    pub name: PatternName
}

#[derive(Copy, Clone, Debug)]
pub struct ReversalPatternParams {
    pub trend_size: usize,
    pub counter_trend_size: usize,
    pub name: PatternName
}

pub fn find_potential_w_pattern(vec: &[MathKLine], options: WPatternParams) -> Option<(WPattern, usize)>{
    let n: usize = options.klines_repetitions;
    let start_index: usize;
    let second_v_index: usize;
    let end_index: usize = 0;
    let neckline_index: usize;
    let lower_price: f64;
    let neckline_price: f64;
    let start_time = vec[0].open_time;
    let end_time: i64 = 0;

    let is_down_test = vec![TestFunction{function: is_down, params: None}];
    let is_up_test = vec![TestFunction{function: is_up, params: None}];


    // Not enough KLines or downward trend
    if vec.len() < n+options.klines_range || test_multiple_klines(vec, n, &is_down_test).is_none() {
        return None;
    }
    
    // Get start of new upward trend
    if let Some(result) = test_multiple_klines(&vec[n..n+options.klines_range], n, &is_up_test) {
        start_index = result + n;
        lower_price = vec[start_index].low;
    } else {
        return None;
    };
    if vec.len() < start_index+options.klines_range {
        return None;
    }

    // Get neckline KLine
    if let Some(result) = test_multiple_klines(&vec[start_index..start_index+options.klines_range], n, &is_down_test) {
        neckline_index = result + start_index;
        neckline_price = vec[neckline_index].high;
    } else {
        return None;
    };
    if vec.len() < neckline_index+options.klines_range {
        return None;
    }

    // Find the continuation on upward trend + check if lower price breaks
    let second_v_test = vec![
        TestFunction{function: is_up, params: None},
        TestFunction{function: is_not_breaking_price_downwards, params: Some(TestParams{price: Some(lower_price), kline: None})}
        ];
    if let Some(result) = test_multiple_klines(&vec[neckline_index..neckline_index+options.klines_range], n, &second_v_test) {
        second_v_index = result + neckline_index;
    } else {
        return None;
    };
    if vec.len() < second_v_index+options.klines_range {
        return None;
    }
    Some((WPattern { start_index, start_time, end_index: second_v_index, end_time, lower_price, neckline_price }, second_v_index))
}

pub fn find_trigger_w_pattern(vec: &[MathKLine], options: WPatternParams, potential_pattern: WPattern, second_v_index: usize) -> Option<WPattern>{
    let end_index: usize;
    let end_time: i64;

    let neckline_break_test = vec![
        TestFunction{function: is_breaking_price_upwards, params: Some(TestParams{price: Some(potential_pattern.neckline_price), kline: None})}
        ];

    if let Some(result) = test_multiple_klines(&vec[second_v_index..second_v_index+options.klines_range], options.klines_repetitions, &neckline_break_test) {
        end_index = result + second_v_index;
        end_time = vec[end_index].close_time;
    } else {
        return None;
    };

    Some(WPattern { 
        start_index: potential_pattern.start_index, 
        start_time: potential_pattern.start_time, 
        end_index, 
        end_time, 
        lower_price: potential_pattern.lower_price, 
        neckline_price: potential_pattern.neckline_price
    })
}

pub fn find_w_pattern(vec: &[MathKLine], options: WPatternParams, potential_only: bool) -> Option<WPattern>{
    if let Some((pattern, second_v_index)) = find_potential_w_pattern(vec, options) {
        if potential_only {
            return Some(pattern); 
        } else {
            return find_trigger_w_pattern(vec, options, pattern, second_v_index); 
        }
    }
    return None;
}

pub fn find_potential_m_pattern(vec: &[MathKLine], options: MPatternParams) -> Option<(MPattern, usize)>{
    let n: usize = options.klines_repetitions;
    let start_index: usize;
    let second_n_index: usize;
    let end_index: usize = 0;
    let neckline_index: usize;
    let higher_price: f64;
    let neckline_price: f64;
    let start_time = vec[0].open_time;
    let end_time: i64 = 0;

    let is_down_test = vec![TestFunction{function: is_down, params: None}];
    let is_up_test = vec![TestFunction{function: is_up, params: None}];

    //println!("1");
    // Not enough KLines or upward trend
    if vec.len() < n+options.klines_range || test_multiple_klines(&vec[0..n], n, &is_up_test).is_none() {
        return None;
    }

    //println!("2");
    
    let uptrend_end = n-1;
    //println!("vec size = {}, uptrend_end = {}", vec.len(), uptrend_end);
    // Get start of new downward trend
    let first_n_test = vec![
        TestFunction{function: is_down, params: None},
        TestFunction{function: is_not_breaking_price_upwards, params: Some(TestParams{price: Some(vec[uptrend_end].high), kline: None})}
        ];
        
    //println!("2.1");
    if let Some(result) = test_multiple_klines(&vec[n..n+n], n, &first_n_test) {
        //println!("2.1.1");
        start_index = n + n - 1;
        //println!("2.1.2");
        higher_price = vec[uptrend_end].high;
        neckline_price = vec[start_index].low;
        //println!("2.1.3");
    } else {
        return None;
    };
    //println!("2.2");
    if vec.len() < start_index+options.klines_range {
        return None;
    }


    //println!("3");

    // Get neckline KLine
    // faut trouver la kline avec le prix le plus haut sans que ca dépasse le prix de la neckline ni du stop loss

    let find_higher_kline_test = vec![
        TestFunction{function: is_higher_than, params: Some(TestParams{price: Some(neckline_price), kline: None})}
        ];
    let find_higher_kline_failing_condition = vec![
        TestFunction{function: is_breaking_price_upwards, params: Some(TestParams{price: Some(higher_price), kline: None})},
        ];
    let find_higher_kline_fast_condition = vec![
        TestFunction{function: is_breaking_price_downwards, params: Some(TestParams{price: Some(neckline_price), kline: None})}
        ];
    if let Some(result) = find_kline(&vec[start_index..options.klines_range], 
        &find_higher_kline_test,
        &find_higher_kline_failing_condition,
        &find_higher_kline_fast_condition) {
            neckline_index = result + start_index;
    } else {
        return None;
    };
    if vec.len() < neckline_index+options.klines_range {
        return None;
    }





    //println!("4");

    // Find the continuation on downward trend + check if higher price breaks
    if test_multiple_klines(&vec[neckline_index+1..neckline_index+1], 1, &is_down_test).is_none() {
        second_n_index = neckline_index + 1;
    } else {
        return None;
    };
    if vec.len() < second_n_index+options.klines_range {
        return None;
    }

    //println!("5");

    //// etape 1
    //println!("\n\nETAPE 1\n");
    //println!("n : {}", n);
    //println!("vec[0] : {:#?}", vec[0]);
    //println!("uptrend_end : {}", uptrend_end);
    //println!("vec[uptrend_end] : {:#?}", vec[uptrend_end]);
    //println!("---------------------------------------------");
    //// etape 2
    //println!("\n\nETAPE 2\n");
    //println!("n : {}", n);
    //println!("vec[uptrend_end].high : {:#?}", vec[uptrend_end].high);
    //println!("start_index : {}", start_index);
    //println!("higher_price : {:#?}", higher_price);
    //println!("vec[start_index] : {:#?}", vec[start_index]);
    //println!("neckline_price : {:#?}", neckline_price);
    //println!("---------------------------------------------");
    ////etape 3
    //println!("\n\nETAPE 3\n");
    //println!("n : {}", n);
    //println!("higher_price : {:#?}", higher_price);
    //println!("start_index : {}", start_index);
    //println!("vec[start_index].low : {:#?}", vec[start_index].low);
    //println!("neckline_index : {:#?}", neckline_index);
    //println!("vec[neckline_index] : {:#?}", vec[neckline_index]);
    //println!("neckline_price : {:#?}", neckline_price);
    //println!("---------------------------------------------");
    ////etape 4
    //println!("\n\nETAPE 4\n");
    //println!("n : {}", n);
    //println!("neckline_index : {:#?}", neckline_index);
    //println!("vec[neckline_index] : {:#?}", vec[neckline_index]);
    //println!("vec[neckline_index].high : {:#?}", vec[neckline_index].high);
    //println!("second_n_index : {:#?}", second_n_index);
    //println!("---------------------------------------------");


    /// ICI TODO FAIRE UN PRINT DE TOUTES LES VARIABLES INTERMEDIRAIRES POUR VERIFIER QUE CA PREND PAS N'IMP (et jsuis a peu près sur que ca prend n'imp)
    //println!("potential m pattern : \n uptrend_end : {},\n start_index : {},\n higher_price : {},\n neckline_index : {},\n neckline_price : {},\n second_n_index : {}", uptrend_end, start_index, higher_price, neckline_index, neckline_price, second_n_index);

    Some((MPattern { start_index, start_time, end_index: second_n_index, end_time, higher_price, neckline_price }, second_n_index))
}

pub fn find_trigger_m_pattern(vec: &[MathKLine], options: MPatternParams, potential_pattern: MPattern, second_n_index: usize) -> Option<MPattern>{
    let end_index: usize;
    let end_time: i64;

    let find_higher_kline_test = vec![
        ];
    let find_higher_kline_failing_condition = vec![
        TestFunction{function: is_breaking_price_upwards, params: Some(TestParams{price: Some(potential_pattern.higher_price), kline: None})},
        ];
    let find_higher_kline_fast_condition = vec![
        TestFunction{function: is_breaking_price_downwards, params: Some(TestParams{price: Some(potential_pattern.neckline_price), kline: None})}
        ];
    if let Some(result) = find_kline(&vec[potential_pattern.start_index..options.klines_range], 
        &find_higher_kline_test,
        &find_higher_kline_failing_condition,
        &find_higher_kline_fast_condition) {
            end_index = result + second_n_index;
            end_time = vec[end_index].close_time;
    } else {
        return None;
    };

    //println!("confirmed M pattern : \n start_index : {}\nstart_time : {}\nend_index : {}\nend_time : {}\nhigher_price : {}\nneckline_price : {}", 
    //potential_pattern.start_index,
    //potential_pattern.start_time,
    //end_index,
    //end_time,
    //potential_pattern.higher_price,
    //potential_pattern.neckline_price);
    Some(MPattern { 
        start_index: potential_pattern.start_index, 
        start_time: potential_pattern.start_time, 
        end_index, 
        end_time, 
        higher_price: potential_pattern.higher_price, 
        neckline_price: potential_pattern.neckline_price
    })
}

pub fn find_m_pattern(vec: &[MathKLine], options: MPatternParams, potential_only: bool) -> Option<MPattern>{
    if let Some((pattern, second_v_index)) = find_potential_m_pattern(vec, options) {
        if potential_only {
            return Some(pattern); 
        } else {
            return find_trigger_m_pattern(vec, options, pattern, second_v_index); 
        }
    }
    return None;
}

pub fn find_bull_reversal(vec: &[MathKLine], options: ReversalPatternParams, potential_only: bool) -> Option<ReversalPattern>{
    let start_index;
    let start_time;
    let end_index;
    let end_time;
    let peak_price;
    let end_price;

    let trend_end_index;

    let is_down_test = vec![TestFunction{function: is_down, params: None}];
    let is_up_test = vec![TestFunction{function: is_up, params: None}];

    if let Some(result) = test_multiple_klines(&vec[0..], options.trend_size, &is_down_test) {
        start_index = 0;
        start_time = vec[0].open_time;
        trend_end_index = result;
        peak_price = vec[result].close;
    } else {
        return None;
    }
    if let Some(result) = test_multiple_klines(&vec[trend_end_index..], options.counter_trend_size, &is_up_test) {
        end_index = result + trend_end_index;
        end_time = vec[end_index].close_time;
        end_price = vec[end_index].close;
    } else {
        return None;
    }
    Some(ReversalPattern { start_index, start_time, end_index, end_time, peak_price, end_price })
}

fn test_multiple_klines(vec: &[MathKLine], repetitions: usize, tests: &[TestFunction]) -> Option<usize> {
    let mut tests_passed = 0;
    let mut klines_ok = 0;

    for (i, item) in vec.iter().enumerate() {
        tests_passed = 0;
        for test in tests {
            if (test.function)(item.clone(), test.params.clone()) {
                tests_passed += 1;
                //println!("tests.len = {}, success_count = {}", tests.len(), tests_passed);
                if tests_passed >= tests.len() {
                    klines_ok += 1;
                    //println!("klines_ok = {}", klines_ok);
                }
            } else {
                klines_ok = 0;
                break;
            }
        }
        if klines_ok >= repetitions {
            //println!("i = {}, klines_ok = {}", i, klines_ok);
            return Some(i-(klines_ok-1));
        }
    }
    None
}

fn find_kline(vec: &[MathKLine], tests: &[TestFunction], failing_conditions: &[TestFunction], early_conditions: &[TestFunction]) -> Option<usize> {
    let mut tests_passed = 0;
    let mut best_kline_index = 0;

    for (i, item) in vec.iter().enumerate() {
        for constraint in failing_conditions {
            //println!("ca va test la failing condition");
            if (constraint.function)(item.clone(), constraint.params.clone()) {
                //println!("ca fail avec constraint.params.price = {}", constraint.params.clone().unwrap().price.unwrap());
                return None;
            }
        }

        for constraint in early_conditions {
            if (constraint.function)(item.clone(), constraint.params.clone()) {
                for test in tests {
                    let mut params = test.params.clone().unwrap();
                    params.kline = Some(vec[best_kline_index].clone());
                    if (test.function)(item.clone(), Some(params)) {
                        best_kline_index = i;
                    }
                }
                //println!("ca passe avec constraint.params.price = {} pour la kline {:#?}", constraint.params.clone().unwrap().price.unwrap(), vec[best_kline_index]);
                return Some(best_kline_index);
            }
        }

        for test in tests {
            let mut params = test.params.clone().unwrap();
            params.kline = Some(vec[best_kline_index].clone());
            if (test.function)(item.clone(), Some(params)) {
                best_kline_index = i;
            }
        }
    }
    return Some(best_kline_index);
}

fn is_up(kline: MathKLine, _: Option<TestParams>) -> bool {
    kline.close > kline.open
}

fn is_down(kline: MathKLine, _: Option<TestParams>) -> bool {
    kline.close < kline.open
}

fn is_breaking_price_upwards(kline: MathKLine, params: Option<TestParams>) -> bool {
    //println!("kline.high : {},  params.price : {}", kline.high, params.clone().unwrap().price.unwrap());
    kline.high > params.unwrap().price.unwrap()
}

fn is_breaking_price_downwards(kline: MathKLine, params: Option<TestParams>) -> bool {
    kline.low < params.unwrap().price.unwrap()
}

fn is_not_breaking_price_upwards(kline: MathKLine, params: Option<TestParams>) -> bool {
    !(kline.high > params.unwrap().price.unwrap())
}

fn is_not_breaking_price_downwards(kline: MathKLine, params: Option<TestParams>) -> bool {
    !(kline.low < params.unwrap().price.unwrap())
}

fn is_higher_than(kline: MathKLine, params: Option<TestParams>) -> bool {
    kline.close > params.unwrap().price.unwrap()
}

fn is_lower_than(kline: MathKLine, params: Option<TestParams>) -> bool {
    kline.close < params.unwrap().price.unwrap()
}


pub unsafe fn _create_test_kline(open: f64, close: f64) -> MathKLine {
    _KLINE_TIME += 1;
    MathKLine{
        open_time: _KLINE_TIME,
        open,
        high: if open > close {open + 0.5} else {close + 0.5},
        low: if open < close {open - 0.5} else {close - 0.5},
        close,
        volume: "".to_string(),
        close_time: _KLINE_TIME+1,
        quote_asset_volume: "".to_string(),
        number_of_trades: 0,
        taker_buy_base_asset_volume: "".to_string(),
        taker_buy_quote_asset_volume: "".to_string()
    }
}