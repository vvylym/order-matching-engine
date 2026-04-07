//! Sharded **multi-book** throughput: parallel add-only workload across independent engines.
//!
//! This is the closest “north star” proxy in-tree today: single-writer per book, scale via shards.

use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use omer::engine::{AddOrderCommand, OrderMatchingService};
use omer::harness::{
    EngineWithBookNoOp, InMemoryPriceBook, engine_with_book_noop,
};
use omer::types::{OrderType, Side, TimeInForce};
use rayon::prelude::*;

const OPS_PER_SHARD: u64 = 100_000;

type ShardEngine = EngineWithBookNoOp<InMemoryPriceBook>;

fn mk_resting_add(id: u64, symbol_id: u32) -> AddOrderCommand {
    AddOrderCommand {
        id,
        participant_id: 100,
        symbol_id,
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(100 + (id as i64 % 400)),
        quantity: 1,
        time_in_force: TimeInForce::Gtc,
        stop_price: None,
        max_visible_quantity: None,
        slippage: None,
        trailing_distance: None,
        trailing_step: None,
    }
}

fn throughput_sharded_add(c: &mut Criterion) {
    let shards = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let mut engines: Vec<ShardEngine> =
        (0..shards).map(|_| engine_with_book_noop()).collect();
    let next_ids: Vec<AtomicU64> = (0..shards)
        .map(|i| AtomicU64::new(1 + (i as u64).wrapping_mul(1_000_000_000)))
        .collect();

    let mut group = c.benchmark_group("throughput_sharded_add");
    group.throughput(Throughput::Elements(OPS_PER_SHARD * shards as u64));

    group.bench_function("inmemory_price_book_noop_sink", |b| {
        b.iter(|| {
            engines
                .par_iter_mut()
                .enumerate()
                .for_each(|(shard_idx, engine)| {
                    let mut id = next_ids[shard_idx].load(Ordering::Relaxed);
                    let symbol_id = shard_idx as u32 + 1;
                    for _ in 0..OPS_PER_SHARD {
                        // NB: symbol id is stable per shard; ids are monotonic per shard.
                        black_box(
                            engine.add(mk_resting_add(id, symbol_id)).is_ok(),
                        );
                        id = id.wrapping_add(1);
                    }
                    next_ids[shard_idx].store(id, Ordering::Relaxed);
                });
        });
    });

    group.finish();
}

criterion_group!(benches, throughput_sharded_add);
criterion_main!(benches);
