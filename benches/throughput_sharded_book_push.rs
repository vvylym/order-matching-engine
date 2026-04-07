//! Sharded book-only throughput: parallel [`PriceBook::push`] across independent books.
//!
//! This is an **upper bound** for shard scaling since it excludes store/policies/events.

use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use omer::book::PriceBook;
use omer::book::service::DashSkipOrderBook;
use omer::types::Side;
use rayon::prelude::*;

const OPS_PER_SHARD: u64 = 200_000;

type Book = DashSkipOrderBook;

fn throughput_sharded_book_push(c: &mut Criterion) {
    let shards = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    // Reuse books across iterations so we measure push hot path, not allocator setup.
    let mut books: Vec<Book> = (0..shards).map(|_| Book::new()).collect();
    let next_order_id: Vec<AtomicU64> =
        (0..shards).map(|i| AtomicU64::new(i as u64)).collect();
    let next_price: Vec<AtomicU64> = (0..shards)
        .map(|i| AtomicU64::new(10_000 + (i as u64) * 1_000_000))
        .collect();

    let mut group = c.benchmark_group("throughput_sharded_book_push");
    group.throughput(Throughput::Elements(OPS_PER_SHARD * shards as u64));

    group.bench_function("dash_skip_push_same_price_fifo", |b| {
        b.iter(|| {
            books
                .par_iter_mut()
                .enumerate()
                .for_each(|(shard_idx, book)| {
                    let mut id = next_order_id[shard_idx].load(Ordering::Relaxed);
                    for _ in 0..OPS_PER_SHARD {
                        book.push(&100, id, Side::Buy, id);
                        id = id.wrapping_add(1);
                    }
                    next_order_id[shard_idx].store(id, Ordering::Relaxed);
                    black_box(book.best_bid());
                });
        });
    });

    group.bench_function("dash_skip_push_distinct_prices", |b| {
        b.iter(|| {
            books
                .par_iter_mut()
                .enumerate()
                .for_each(|(shard_idx, book)| {
                    let mut id = next_order_id[shard_idx].load(Ordering::Relaxed);
                    let mut p = next_price[shard_idx].load(Ordering::Relaxed);
                    for _ in 0..OPS_PER_SHARD {
                        // Monotonic prices within a shard to avoid intra-level FIFO effects.
                        book.push(&(p as i64), id, Side::Buy, id);
                        id = id.wrapping_add(1);
                        p = p.wrapping_add(1);
                    }
                    next_order_id[shard_idx].store(id, Ordering::Relaxed);
                    next_price[shard_idx].store(p, Ordering::Relaxed);
                    black_box(book.best_bid());
                });
        });
    });

    group.finish();
}

criterion_group!(benches, throughput_sharded_book_push);
criterion_main!(benches);
