//! Long **deterministic pseudo-random** command stream; asserts book never crosses (best bid ≤ best ask).

mod util;

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use omer::engine::{CancelByOrderIdCommand, OrderMatchingService};
use omer::types::{OrderType, Side, TimeInForce};

use util::{MinimalNoopEngine, minimal_noop_engine};

use omer::engine::AddOrderCommand;

fn lim(id: u64, side: Side, price: i64, q: i64, part: u64) -> AddOrderCommand {
    AddOrderCommand {
        id,
        participant_id: part,
        symbol_id: 1,
        side,
        order_type: OrderType::Limit,
        price: Some(price),
        quantity: q,
        time_in_force: TimeInForce::Gtc,
        stop_price: None,
        max_visible_quantity: None,
        slippage: None,
        trailing_distance: None,
        trailing_step: None,
    }
}

fn assert_uncrossed(eng: &MinimalNoopEngine) {
    match (eng.best_bid(), eng.best_ask()) {
        (Some(b), Some(a)) if b > a => panic!("crossed book bid={b} ask={a}"),
        _ => {}
    }
}

fn integrity_stress_once(events: u64) {
    let mut eng = minimal_noop_engine();
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next_id: u64 = 1;
    for _ in 0..events {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let op = state % 5;
        match op {
            0 | 1 => {
                let id = next_id;
                next_id += 1;
                let price = 40 + (state % 120) as i64;
                let _ = eng.add(lim(
                    id,
                    if state & 1 == 0 {
                        Side::Buy
                    } else {
                        Side::Sell
                    },
                    price,
                    1 + (state % 5) as i64,
                    100 + (state % 3),
                ));
            }
            2 if next_id > 1 => {
                let span = next_id - 1;
                let victim = 1 + (state % span);
                let _ = eng.cancel_by_order_id(CancelByOrderIdCommand {
                    order_id: victim,
                });
            }
            2 => {}
            3 => {
                let id = next_id;
                next_id += 1;
                let _ = eng.add(lim(id, Side::Sell, 200, 2, 200));
                let m = next_id;
                next_id += 1;
                let _ = eng.add(AddOrderCommand {
                    id: m,
                    participant_id: 201,
                    symbol_id: 1,
                    side: Side::Buy,
                    order_type: OrderType::Market,
                    price: None,
                    quantity: 1,
                    time_in_force: TimeInForce::Ioc,
                    stop_price: None,
                    max_visible_quantity: None,
                    slippage: None,
                    trailing_distance: None,
                    trailing_step: None,
                });
            }
            _ => {
                let id = next_id;
                next_id += 1;
                let _ = eng.add(lim(id, Side::Buy, 55, 2, 300));
            }
        }
        assert_uncrossed(&eng);
    }
    black_box(next_id);
}

fn integrity_stress(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrity_stress");
    const N: u64 = 16_384;
    group.throughput(Throughput::Elements(N));
    group.bench_function("randomish_ops_uncrossed", |b| {
        b.iter(|| integrity_stress_once(N));
    });
    group.finish();
}

criterion_group!(benches, integrity_stress);
criterion_main!(benches);
