//! Sharded mixed throughput with explicit `OrderId -> shard` routing.
//!
//! This models gateway-side routing for cancel-heavy flows:
//! adds are routed by symbol/shard, and cancels use the routing index.

use std::collections::{HashMap, VecDeque};
use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use omer::engine::{
    AddOrderCommand, CancelByOrderIdCommand, OrderCommand, OrderMatchingService,
};
use omer::harness::{
    EngineWithBookNoOp, InMemoryPriceBook, engine_with_book_noop,
};
use omer::types::{OrderType, Side, TimeInForce};
use rayon::prelude::*;

const OPS_PER_SHARD: u64 = 20_000;

type ShardEngine = EngineWithBookNoOp<InMemoryPriceBook>;
type AliveQueues = Vec<VecDeque<u64>>;
type RoutedBatches = Vec<Vec<OrderCommand>>;

struct ShardedRouter {
    engines: Vec<ShardEngine>,
    order_to_shard: HashMap<u64, usize>,
    alive_ids: AliveQueues,
    next_order_id: u64,
}

impl ShardedRouter {
    fn new(shards: usize) -> Self {
        Self {
            engines: (0..shards).map(|_| engine_with_book_noop()).collect(),
            order_to_shard: HashMap::with_capacity(shards * 8_192),
            alive_ids: (0..shards).map(|_| VecDeque::new()).collect(),
            next_order_id: 1,
        }
    }

    fn alloc_order_id(&mut self) -> u64 {
        let id = self.next_order_id;
        self.next_order_id = self.next_order_id.wrapping_add(1);
        id
    }

    fn add_limit(
        &mut self,
        shard: usize,
        side: Side,
        qty: i64,
        price: i64,
        batches: &mut RoutedBatches,
    ) {
        let id = self.alloc_order_id();
        self.order_to_shard.insert(id, shard);
        self.alive_ids[shard].push_back(id);

        batches[shard].push(OrderCommand::Add(AddOrderCommand {
            id,
            participant_id: 100 + shard as u64,
            symbol_id: shard as u32 + 1,
            side,
            order_type: OrderType::Limit,
            price: Some(price),
            quantity: qty,
            time_in_force: TimeInForce::Gtc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
        }));
    }

    fn add_market(
        &mut self,
        shard: usize,
        side: Side,
        qty: i64,
        batches: &mut RoutedBatches,
    ) {
        let id = self.alloc_order_id();
        batches[shard].push(OrderCommand::Add(AddOrderCommand {
            id,
            participant_id: 200 + shard as u64,
            symbol_id: shard as u32 + 1,
            side,
            order_type: OrderType::Market,
            price: None,
            quantity: qty,
            time_in_force: TimeInForce::Ioc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
        }));
    }

    fn cancel_oldest(&mut self, shard: usize, batches: &mut RoutedBatches) {
        if let Some(order_id) = self.alive_ids[shard].pop_front() {
            self.order_to_shard.remove(&order_id);
            batches[shard].push(OrderCommand::CancelByOrderId(
                CancelByOrderIdCommand { order_id },
            ));
        }
    }

    fn run_round(&mut self) {
        let shards = self.engines.len();
        let mut batches: RoutedBatches = vec![Vec::<OrderCommand>::new(); shards];

        for shard in 0..shards {
            for i in 0..OPS_PER_SHARD {
                match i % 10 {
                    0..=5 => {
                        let p = 100 + ((i as i64 + shard as i64) % 80);
                        self.add_limit(shard, Side::Buy, 1, p, &mut batches);
                    }
                    6 | 7 => self.cancel_oldest(shard, &mut batches),
                    8 => self.add_limit(shard, Side::Sell, 5, 250, &mut batches),
                    _ => self.add_market(shard, Side::Buy, 5, &mut batches),
                }
            }
        }

        self.engines
            .par_iter_mut()
            .zip(batches.into_par_iter())
            .for_each(|(engine, cmds)| {
                for cmd in cmds {
                    let _ = engine.process(cmd);
                }
                black_box((engine.best_bid(), engine.best_ask()));
            });
    }
}

fn throughput_sharded_mixed(c: &mut Criterion) {
    let shards = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let mut router = ShardedRouter::new(shards);

    let mut group = c.benchmark_group("throughput_sharded_mixed");
    group.throughput(Throughput::Elements(OPS_PER_SHARD * shards as u64));
    group.bench_function("order_id_indexed_add_cancel_market", |b| {
        b.iter(|| {
            router.run_round();
        });
    });
    group.finish();
}

criterion_group!(benches, throughput_sharded_mixed);
criterion_main!(benches);
