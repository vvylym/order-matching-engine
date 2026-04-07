//! Compares **noop** vs **collecting** event sink on the same resting-add workload.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use omer::engine::OrderMatchingService;
use omer::harness::{
    InMemoryPriceBook, add_cmd, engine_with_book_noop, engine_with_memory,
};
use omer::types::{OrderType, Side, TimeInForce};

fn observability_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("observability_overhead_add");
    group.bench_function("noop_sink", |b| {
        let mut engine = engine_with_book_noop::<InMemoryPriceBook>();
        let mut next_id = 1_u64;
        b.iter(|| {
            let price = 100_i64 + (next_id as i64 % 47);
            black_box(
                engine
                    .add(add_cmd(
                        next_id,
                        100,
                        Side::Buy,
                        OrderType::Limit,
                        Some(price),
                        1,
                        TimeInForce::Gtc,
                    ))
                    .is_ok(),
            );
            next_id = next_id.wrapping_add(1);
        });
    });
    group.bench_function("collecting_sink", |b| {
        let (mut engine, _sink) = engine_with_memory();
        let mut next_id = 1_u64;
        b.iter(|| {
            let price = 100_i64 + (next_id as i64 % 47);
            black_box(
                engine
                    .add(add_cmd(
                        next_id,
                        100,
                        Side::Buy,
                        OrderType::Limit,
                        Some(price),
                        1,
                        TimeInForce::Gtc,
                    ))
                    .is_ok(),
            );
            next_id = next_id.wrapping_add(1);
        });
    });
    group.finish();
}

criterion_group!(benches, observability_overhead);
criterion_main!(benches);
