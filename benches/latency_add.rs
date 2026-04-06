//! Latency: resting limit add (`engine.add`) via [`omer::harness`].

use criterion::{Criterion, criterion_group, criterion_main};
use omer::engine::OrderMatchingService;
use omer::harness::{add_cmd, engine_with_memory};
use omer::types::{OrderType, Side, TimeInForce};
use std::hint::black_box;

fn latency_add_resting_limit(c: &mut Criterion) {
    let (mut engine, _sink) = engine_with_memory();
    let mut next_id = 1_u64;
    c.bench_function("latency_add_limit_resting", |b| {
        b.iter(|| {
            let price = 100_i64 + (next_id as i64 % 47);
            let cmd = add_cmd(
                next_id,
                100,
                Side::Buy,
                OrderType::Limit,
                Some(price),
                1,
                TimeInForce::Gtc,
            );
            next_id = next_id.wrapping_add(1);
            black_box(engine.add(cmd).is_ok());
        });
    });
}

criterion_group!(benches, latency_add_resting_limit);
criterion_main!(benches);
