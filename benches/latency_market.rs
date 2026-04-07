//! Latency: **resting limit + market IOC** (full cross) per iteration, noop sink.

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

fn bench_market_cross<PB: PriceBook + Default>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    label: &'static str,
) {
    let mut engine = engine_with_book_noop::<PB>();
    let mut resting_id = 1_u64;
    let mut market_id = 2_u64;
    group.bench_function(label, |b| {
        b.iter(|| {
            let r = resting_id;
            let m = market_id;
            resting_id = resting_id.wrapping_add(2);
            market_id = market_id.wrapping_add(2);
            engine
                .add(add_cmd(
                    r,
                    100,
                    Side::Sell,
                    OrderType::Limit,
                    Some(50),
                    10,
                    TimeInForce::Gtc,
                ))
                .unwrap();
            black_box(
                engine
                    .add(add_cmd(
                        m,
                        101,
                        Side::Buy,
                        OrderType::Market,
                        None,
                        10,
                        TimeInForce::Ioc,
                    ))
                    .is_ok(),
            );
        });
    });
}

fn latency_market(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_market_limit_cross");
    bench_market_cross::<InMemoryPriceBook>(&mut group, "inmemory_price_book");
    bench_market_cross::<BTreeOrderBook>(&mut group, "btree_order_book");
    bench_market_cross::<PoolLevelOrderBook>(&mut group, "pool_level_book");
    bench_market_cross::<DashSkipOrderBook>(&mut group, "dash_skip_book");
    group.finish();
}

criterion_group!(benches, latency_market);
criterion_main!(benches);
