//! Multi-book / “market manager” style bench (placeholder).
//!
//! **Goal:** ITCH (or similar) into a handler without matching, reporting msg/s and book stats.
//! Not wired yet: the loop only uses `black_box`.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn market_manager_itch_no_matching(c: &mut Criterion) {
    // Stub. Real benchmark will:
    // - load ITCH file
    // - MarketManager with matching disabled
    // - DefaultItchHandler(manager)
    // - process(buffer, handler)
    // - report msg/s, updates/s, max symbols/orders/levels
    c.bench_function("market_manager_itch_no_matching", |b| {
        b.iter(|| {
            black_box(0u64);
        });
    });
}

criterion_group!(benches, market_manager_itch_no_matching);
criterion_main!(benches);
