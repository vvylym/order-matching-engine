//! Market manager benchmark (target).
//!
//! **Target:** Run ITCH through parser + handler that only does Add/Reduce/Delete/Replace/Execute
//! (no matching). Report: total time, ITCH msg/s, market updates/s, max symbols, max orders, max levels.
//! CppTrader reference: ~3.2M msg/s, ~7.2M upd/s. Until manager + ITCH exist, this is a no-op stub.

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
