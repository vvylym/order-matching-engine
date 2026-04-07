//! Sustained **mixed** workload from an empty book each iteration: mostly adds + cancel-by-id + occasional market cross.

use std::collections::VecDeque;
use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use omer::engine::{CancelByOrderIdCommand, OrderMatchingService};
use omer::harness::{
    EngineWithBookNoOp, InMemoryPriceBook, add_cmd, engine_with_book_noop,
};
use omer::types::{OrderType, Side, TimeInForce};

const OPS: u64 = 512;

fn run_mixed_round(engine: &mut EngineWithBookNoOp<InMemoryPriceBook>) {
    let mut next_id: u64 = 1;
    let mut alive: VecDeque<u64> = VecDeque::new();
    let mut resting_sell: Option<u64> = None;

    for i in 0..OPS {
        match i % 10 {
            0..=5 => {
                let id = next_id;
                next_id += 1;
                let price = 100 + (id as i64 % 80);
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
                alive.push_back(id);
            }
            6 | 7 => {
                if let Some(victim) = alive.pop_front() {
                    let _ = engine.cancel_by_order_id(CancelByOrderIdCommand {
                        order_id: victim,
                    });
                }
            }
            8 => {
                let id = next_id;
                next_id += 1;
                engine
                    .add(add_cmd(
                        id,
                        100,
                        Side::Sell,
                        OrderType::Limit,
                        Some(250),
                        5,
                        TimeInForce::Gtc,
                    ))
                    .unwrap();
                resting_sell = Some(id);
            }
            _ => {
                if let Some(_r) = resting_sell.take() {
                    let m = next_id;
                    next_id += 1;
                    let _ = engine.add(add_cmd(
                        m,
                        101,
                        Side::Buy,
                        OrderType::Market,
                        None,
                        5,
                        TimeInForce::Ioc,
                    ));
                }
            }
        }
    }
}

fn throughput_mixed_inmemory(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_mixed_inmemory");
    group.throughput(Throughput::Elements(OPS));
    group.bench_function("mixed_add_cancel_market", |b| {
        b.iter(|| {
            let mut engine = engine_with_book_noop::<InMemoryPriceBook>();
            run_mixed_round(&mut engine);
            black_box((engine.best_bid(), engine.best_ask()));
        });
    });
    group.finish();
}

criterion_group!(benches, throughput_mixed_inmemory);
criterion_main!(benches);
