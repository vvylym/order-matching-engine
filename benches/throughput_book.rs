//! Throughput: [`PriceBook::push`] (order id + side + sequence only; no `Order` clone) on
//! [`omer::book::service::DashSkipOrderBook`] ([`dashmap`] + [`crossbeam_skiplist`]).
//!
//! Record results with CPU model and `rustc -V`; CI only compiles benches (`cargo bench --no-run`).
//!
//! [`dashmap`]: https://docs.rs/dashmap
//! [`crossbeam_skiplist`]: https://docs.rs/crossbeam-skiplist

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use omer::book::PriceBook;
use omer::book::service::DashSkipOrderBook;
use omer::types::Side;
use std::hint::black_box;

const N: u64 = 50_000;

fn throughput_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("dash_skip_book");
    group.throughput(Throughput::Elements(N));

    group.bench_function("push_same_price_buy_fifo", |b| {
        b.iter(|| {
            let mut book = DashSkipOrderBook::new();
            for i in 0..N {
                book.push(&100, i, Side::Buy, i);
            }
            black_box(book.best_bid());
        });
    });

    group.bench_function("push_distinct_prices_buy", |b| {
        b.iter(|| {
            let mut book = DashSkipOrderBook::new();
            for i in 0..N {
                let p = 100 + i as i64;
                book.push(&p, i, Side::Buy, i);
            }
            black_box(book.best_bid());
        });
    });

    group.finish();
}

criterion_group!(benches, throughput_push);
criterion_main!(benches);
