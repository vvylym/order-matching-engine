//! **Adversarial** pattern: thousands of distinct sell price levels on [`DashSkipOrderBook`], then market sweeps.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use omer::book::service::DashSkipOrderBook;
use omer::engine::OrderMatchingService;
use omer::harness::{add_cmd, engine_with_book_noop};
use omer::types::{OrderType, Side, TimeInForce};

const LEVELS: u64 = 5_000;
const SWEEPS: u64 = 256;

fn throughput_adversarial(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_adversarial");
    group.throughput(Throughput::Elements(LEVELS + SWEEPS));
    group.bench_function("dash_skip_deep_then_market_sweeps", |b| {
        b.iter(|| {
            let mut engine = engine_with_book_noop::<DashSkipOrderBook>();
            for i in 1..=LEVELS {
                engine
                    .add(add_cmd(
                        i,
                        100,
                        Side::Sell,
                        OrderType::Limit,
                        Some(10_000 + i as i64),
                        1,
                        TimeInForce::Gtc,
                    ))
                    .unwrap();
            }
            for j in 0..SWEEPS {
                let m = LEVELS + 1 + j;
                black_box(
                    engine
                        .add(add_cmd(
                            m,
                            101,
                            Side::Buy,
                            OrderType::Market,
                            None,
                            1,
                            TimeInForce::Ioc,
                        ))
                        .is_ok(),
                );
            }
        });
    });
    group.finish();
}

criterion_group!(benches, throughput_adversarial);
criterion_main!(benches);
