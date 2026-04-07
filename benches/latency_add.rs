//! Latency: resting limit add (`engine.add`) with **[`NoOpEventSink`](omer::events::NoOpEventSink)**,
//! varying only [`PriceBook`] (same as other `latency_*` benches).

use criterion::{
    BenchmarkGroup, Criterion, criterion_group, criterion_main,
    measurement::WallTime,
};
use omer::book::PriceBook;
use omer::book::service::{
    BTreeOrderBook, DashSkipOrderBook, PoolLevelOrderBook,
};
use omer::engine::OrderMatchingService;
use omer::harness::{InMemoryPriceBook, add_cmd, engine_with_book_noop};
use omer::types::{OrderType, Side, TimeInForce};
use std::hint::black_box;

fn bench_add<PB: PriceBook + Default>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    label: &'static str,
) {
    let mut engine = engine_with_book_noop::<PB>();
    let mut next_id = 1_u64;
    group.bench_function(label, |b| {
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

fn latency_add_limit_resting(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_add_limit_resting");
    bench_add::<InMemoryPriceBook>(&mut group, "inmemory_price_book");
    bench_add::<BTreeOrderBook>(&mut group, "btree_order_book");
    bench_add::<PoolLevelOrderBook>(&mut group, "pool_level_book");
    bench_add::<DashSkipOrderBook>(&mut group, "dash_skip_book");
    group.finish();
}

criterion_group!(benches, latency_add_limit_resting);
criterion_main!(benches);
