//! Throughput: [`PriceBook::push`] on [`omer::book::service::DashSkipOrderBook`]
//! (Phase 1 — [`dashmap`] + [`crossbeam_skiplist`]).
//!
//! Local target band (single thread, no matching engine): **~200–300M pushes/sec** on strong
//! desktop/server CPUs — verify on your machine; report CPU model and `rustc -V`.
//!
//! [`dashmap`]: https://docs.rs/dashmap
//! [`crossbeam_skiplist`]: https://docs.rs/crossbeam-skiplist

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use omer::book::PriceBook;
use omer::book::service::DashSkipOrderBook;
use omer::types::{Order, OrderId, OrderType, Side, TimeInForce};
use std::hint::black_box;

fn mk_order(id: OrderId, side: Side, price: i64, qty: i64) -> Order {
    Order {
        symbol_id: 0,
        id,
        participant_id: 0,
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
        executed_quantity: 0,
        leaves_quantity: qty,
        sequence: id,
    }
}

const N: u64 = 50_000;

fn throughput_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("dash_skip_book");
    group.throughput(Throughput::Elements(N));

    group.bench_function("push_same_price_buy_fifo", |b| {
        b.iter(|| {
            let mut book = DashSkipOrderBook::new();
            for i in 0..N {
                let o = mk_order(i, Side::Buy, 100, 1);
                book.push(&100, &o);
            }
            black_box(book.best_bid());
        });
    });

    group.bench_function("push_distinct_prices_buy", |b| {
        b.iter(|| {
            let mut book = DashSkipOrderBook::new();
            for i in 0..N {
                let p = 100 + i as i64;
                let o = mk_order(i, Side::Buy, p, 1);
                book.push(&p, &o);
            }
            black_box(book.best_bid());
        });
    });

    group.finish();
}

criterion_group!(benches, throughput_push);
criterion_main!(benches);
