//! Latency: **add + cancel-by-id** cycle per iteration (noop sink). Cancel alone needs a fresh order each time.

use criterion::{
    BenchmarkGroup, Criterion, criterion_group, criterion_main,
    measurement::WallTime,
};
use omer::book::PriceBook;
use omer::book::service::{
    BTreeOrderBook, DashSkipOrderBook, PoolLevelOrderBook,
};
use omer::engine::{CancelByOrderIdCommand, OrderMatchingService};
use omer::harness::{InMemoryPriceBook, add_cmd, engine_with_book_noop};
use omer::types::{OrderType, Side, TimeInForce};
use std::hint::black_box;

fn bench_cancel_cycle<PB: PriceBook + Default>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    label: &'static str,
) {
    let mut engine = engine_with_book_noop::<PB>();
    let mut next_id = 1_u64;
    group.bench_function(label, |b| {
        b.iter(|| {
            let id = next_id;
            next_id = next_id.wrapping_add(1);
            let price = 100_i64 + (id as i64 % 47);
            engine
                .add(add_cmd(
                    id,
                    100,
                    Side::Buy,
                    OrderType::Limit,
                    Some(price),
                    1,
                    TimeInForce::Gtc,
                ))
                .unwrap();
            black_box(
                engine
                    .cancel_by_order_id(CancelByOrderIdCommand { order_id: id })
                    .is_ok(),
            );
        });
    });
}

fn latency_cancel(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_cancel_add_cycle");
    bench_cancel_cycle::<InMemoryPriceBook>(&mut group, "inmemory_price_book");
    bench_cancel_cycle::<BTreeOrderBook>(&mut group, "btree_order_book");
    bench_cancel_cycle::<PoolLevelOrderBook>(&mut group, "pool_level_book");
    bench_cancel_cycle::<DashSkipOrderBook>(&mut group, "dash_skip_book");
    group.finish();
}

criterion_group!(benches, latency_cancel);
criterion_main!(benches);
