//! Micro-benchmarks (placeholder).
//!
//! **Goal:** Nanoseconds per add, cancel, replace, and one match step. Use
//! `latency_add` for a real add measurement today; these stubs still call `black_box` only.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn micro_add_order(c: &mut Criterion) {
    // Stub. Real: manager.add(AddOrderCommand { ... }); report ns/op.
    c.bench_function("micro_add_order", |b| b.iter(|| black_box(0u64)));
}

fn micro_cancel(c: &mut Criterion) {
    // Stub. Real: manager.cancel_order(...); report ns/op.
    c.bench_function("micro_cancel", |b| b.iter(|| black_box(0u64)));
}

fn micro_replace(c: &mut Criterion) {
    // Stub. Real: manager.replace_order(...); report ns/op.
    c.bench_function("micro_replace", |b| b.iter(|| black_box(0u64)));
}

fn micro_match_step(c: &mut Criterion) {
    // Stub. Real: one iteration of matching loop (pop best bid/ask, match, update); report ns/op.
    c.bench_function("micro_match_step", |b| b.iter(|| black_box(0u64)));
}

criterion_group!(
    benches,
    micro_add_order,
    micro_cancel,
    micro_replace,
    micro_match_step
);
criterion_main!(benches);
