//! ITCH + matching throughput (placeholder).
//!
//! **Goal:** Feed ITCH (or fixtures) into an engine with matching on, then report rates and stats.
//! Not wired yet: the loop only uses `black_box`.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn matching_engine_itch_with_matching(c: &mut Criterion) {
    // Stub. Real benchmark will:
    // - load ITCH file
    // - MarketManager with matching enabled
    // - DefaultItchHandler(manager)
    // - process(buffer, handler)
    // - report same metrics as market_manager bench
    c.bench_function("matching_engine_itch_with_matching", |b| {
        b.iter(|| {
            black_box(0u64);
        });
    });
}

criterion_group!(benches, matching_engine_itch_with_matching);
criterion_main!(benches);
