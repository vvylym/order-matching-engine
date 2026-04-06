//! Sustained **engine** throughput: warm book + [`OrderMatchingEngine::process_batch`].
//!
//! [`NoOpEventSink`](omer::events::NoOpEventSink) avoids allocator traffic from event collection.
//! Phase 2: compare backends on the **same** workload; Phase 4 scales further (shard, SIMD, affinity).

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use omer::book::service::{
    BTreeOrderBook, DashSkipOrderBook, PoolLevelOrderBook,
};
use omer::engine::{OrderCommand, OrderMatchingService};
use omer::harness::{InMemoryPriceBook, add_cmd, engine_with_book_noop};
use omer::types::{OrderType, Side, TimeInForce};

const WARM_UP: u64 = 4096;
const CHUNK: u64 = 512;

macro_rules! bench_hot_process_batch {
    ($group:ident, $pb:ty, $label:expr) => {{
        let mut engine = engine_with_book_noop::<$pb>();
        let mut id = 1_u64;
        for _ in 0..WARM_UP {
            OrderMatchingService::add(
                &mut engine,
                add_cmd(
                    id,
                    100,
                    Side::Buy,
                    OrderType::Limit,
                    Some(100 + (id as i64 % 400)),
                    1,
                    TimeInForce::Gtc,
                ),
            )
            .unwrap();
            id += 1;
        }
        $group.bench_function($label, |b| {
            b.iter(|| {
                let cmds: Vec<OrderCommand> = (0..CHUNK)
                    .map(|j| {
                        let k = id.wrapping_add(j);
                        OrderCommand::Add(add_cmd(
                            k,
                            100,
                            Side::Buy,
                            OrderType::Limit,
                            Some(100 + (k as i64 % 400)),
                            1,
                            TimeInForce::Gtc,
                        ))
                    })
                    .collect();
                id = id.wrapping_add(CHUNK);
                black_box(engine.process_batch(cmds).unwrap());
            });
        });
    }};
}

fn throughput_engine_hot_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_engine_hot_batch");
    group.throughput(Throughput::Elements(CHUNK));

    bench_hot_process_batch!(group, InMemoryPriceBook, "inmemory_price_book");
    bench_hot_process_batch!(group, BTreeOrderBook, "btree_order_book");
    bench_hot_process_batch!(group, PoolLevelOrderBook, "pool_level_book");
    bench_hot_process_batch!(group, DashSkipOrderBook, "dash_skip_book");

    group.finish();
}

criterion_group!(benches, throughput_engine_hot_batch);
criterion_main!(benches);
