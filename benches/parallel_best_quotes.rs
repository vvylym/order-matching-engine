//! Multi-book **read aggregation**: [`par_best_quotes`](omer::parallel::par_best_quotes) vs sequential scan.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use omer::book::PriceBook;
use omer::book::service::BTreeOrderBook;
use omer::parallel::par_best_quotes;
use omer::types::Side;

const N_BOOKS: usize = 256;

fn build_books() -> Vec<BTreeOrderBook> {
    (0..N_BOOKS)
        .map(|i| {
            let mut b = BTreeOrderBook::new();
            b.push(&(1000 + i as i64), 1 + i as u64, Side::Buy, 0);
            b
        })
        .collect()
}

fn parallel_best_quotes_bench(c: &mut Criterion) {
    let books = build_books();
    let mut group = c.benchmark_group("multi_book_best_quotes");
    group.throughput(Throughput::Elements(N_BOOKS as u64));

    group.bench_function("sequential_map", |b| {
        b.iter(|| {
            let v: Vec<_> = books
                .iter()
                .map(|book| (book.best_bid(), book.best_ask()))
                .collect();
            black_box(v);
        });
    });

    group.bench_function("par_best_quotes_rayon", |b| {
        b.iter(|| {
            black_box(par_best_quotes(black_box(&books)));
        });
    });

    group.finish();
}

criterion_group!(benches, parallel_best_quotes_bench);
criterion_main!(benches);
