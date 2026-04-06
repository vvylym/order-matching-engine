//! Latency: resting limit add (`engine.add`) on the **same harness**, varying only [`PriceBook`].

use criterion::{Criterion, criterion_group, criterion_main};
use omer::engine::OrderMatchingService;
use omer::harness::{
    add_cmd, engine_with_btree_book, engine_with_dash_skip_book,
    engine_with_memory, engine_with_pool_level_book,
};
use omer::types::{OrderType, Side, TimeInForce};
use std::hint::black_box;

fn latency_add_limit_resting(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_add_limit_resting");

    {
        let (mut engine, _sink) = engine_with_memory();
        let mut next_id = 1_u64;
        group.bench_function("inmemory_price_book", |b| {
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

    {
        let (mut engine, _sink) = engine_with_btree_book();
        let mut next_id = 1_u64;
        group.bench_function("btree_order_book", |b| {
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

    {
        let (mut engine, _sink) = engine_with_pool_level_book();
        let mut next_id = 1_u64;
        group.bench_function("pool_level_book", |b| {
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

    {
        let (mut engine, _sink) = engine_with_dash_skip_book();
        let mut next_id = 1_u64;
        group.bench_function("dash_skip_book", |b| {
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

    group.finish();
}

criterion_group!(benches, latency_add_limit_resting);
criterion_main!(benches);
