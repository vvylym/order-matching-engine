//! Cost of **deterministic replay** (engine rebuild + fixed command script + top-of-book checksum).

mod util;

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use omer::engine::{AddOrderCommand, OrderMatchingService};
use omer::types::{OrderType, Side, TimeInForce};

use util::minimal_noop_engine;

fn lim_buy(id: u64, p: i64, q: i64) -> AddOrderCommand {
    AddOrderCommand {
        id,
        participant_id: 100,
        symbol_id: 1,
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(p),
        quantity: q,
        time_in_force: TimeInForce::Gtc,
        stop_price: None,
        max_visible_quantity: None,
        slippage: None,
        trailing_distance: None,
        trailing_step: None,
    }
}

fn lim_sell(id: u64, p: i64, q: i64) -> AddOrderCommand {
    AddOrderCommand {
        id,
        participant_id: 101,
        symbol_id: 1,
        side: Side::Sell,
        order_type: OrderType::Limit,
        price: Some(p),
        quantity: q,
        time_in_force: TimeInForce::Gtc,
        stop_price: None,
        max_visible_quantity: None,
        slippage: None,
        trailing_distance: None,
        trailing_step: None,
    }
}

fn replay_once() -> i64 {
    let mut eng = minimal_noop_engine();
    eng.add(lim_buy(1, 48, 10)).unwrap();
    eng.add(lim_sell(2, 52, 5)).unwrap();
    eng.add(lim_buy(3, 52, 3)).unwrap();
    let bb = eng.best_bid().unwrap_or(0);
    let ba = eng.best_ask().unwrap_or(0);
    bb ^ ba
}

fn correctness_replays(c: &mut Criterion) {
    let mut group = c.benchmark_group("correctness_replay");
    group.bench_function("checksum_256_replays", |b| {
        b.iter(|| {
            let mut acc: i64 = 0;
            for _ in 0..256 {
                acc ^= replay_once();
            }
            black_box(acc);
        });
    });
    group.finish();
}

criterion_group!(benches, correctness_replays);
criterion_main!(benches);
