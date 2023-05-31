use std::{fs::File, io::Write};
use binance::{market::Market, model::{KlineSummary, KlineSummaries}};

pub fn retreive_test_data(
    server_time: u64,
    market: &Market,
    symbol: String,
    interval: String,
    folder: String,
    iterations: usize,
    batch_size: u16,
) -> Vec<KlineSummary> {
    let mut i: u64 = iterations as u64;
    let start_i = i;
    let mut j = 0;
    let mut start_time = server_time - (i * 60 * 1000 * 1000);
    let mut end_time = server_time - ((i - 1) * 60 * 1000 * 1000);

    let mut klines = Vec::new();
    while let Ok(retreive_klines) = market.get_klines(
        symbol.clone(),
        interval.clone(),
        batch_size,
        start_time,
        end_time,
    ) {
        if i == 0 {
            break;
        }
        if let KlineSummaries::AllKlineSummaries(mut retreived_vec) = retreive_klines {
            klines.append(&mut retreived_vec);
        }

        start_time = end_time + 1000 * 60;
        end_time = start_time + 60 * 1000 * 1000;

        i -= 1;
        j += 1;
        if i % 10 == 0 {
            println!("Retreived {}/{} bench of klines data", j, start_i);
        }
    }

    klines
}

pub fn write_data_to_file(
    klines: &Vec<KlineSummary>,
    symbol: String,
    interval: String,
    folder: String,
) {
    let serialized = serde_json::to_string_pretty(klines).unwrap();
    let mut file = File::create(format!(
        "{}{}-{}.json",
        folder, symbol, interval
    ))
    .unwrap();
    file.write_all(serialized.as_bytes()).unwrap();
}