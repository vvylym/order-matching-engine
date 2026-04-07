//! Latency: **add + replace** (in-place replace) per iteration, noop sink.

use criterion::{
    BenchmarkGroup, Criterion, criterion_group, criterion_main,
    measurement::WallTime,
};
use omer::book::PriceBook;
use omer::book::service::{
    BTreeOrderBook, DashSkipOrderBook, PoolLevelOrderBook,
};
use omer::engine::{OrderMatchingService, ReplaceOrderCommand};
use omer::harness::{InMemoryPriceBook, add_cmd, engine_with_book_noop};
use omer::types::{OrderType, Side, TimeInForce};
use std::hint::black_box;

fn bench_replace_after_add<PB: PriceBook + Default>(
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
                    10,
                    TimeInForce::Gtc,
                ))
                .unwrap();
            black_box(
                engine
                    .replace(ReplaceOrderCommand {
                        order_id: id,
                        participant_id: 100,
                        new_price: Some(price.saturating_add(1)),
                        new_quantity: 8,
                        sequence: 0,
                    })
                    .is_ok(),
            );
        });
    });
}

fn latency_replace(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_replace_after_add");
    bench_replace_after_add::<InMemoryPriceBook>(
        &mut group,
        "inmemory_price_book",
    );
    bench_replace_after_add::<BTreeOrderBook>(&mut group, "btree_order_book");
    bench_replace_after_add::<PoolLevelOrderBook>(&mut group, "pool_level_book");
    bench_replace_after_add::<DashSkipOrderBook>(&mut group, "dash_skip_book");
    group.finish();
}

criterion_group!(benches, latency_replace);
criterion_main!(benches);
