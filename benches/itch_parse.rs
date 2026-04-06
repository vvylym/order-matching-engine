//! ITCH parse benchmark (target).
//!
//! **Target:** Parse a fixed ITCH buffer (or file), report messages per second and ns per message.
//! CppTrader reference: ~41M msg/s. Until parser exists, this is a no-op stub.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn itch_parse_throughput(c: &mut Criterion) {
    // Stub: no parser yet. Real benchmark will:
    // - load buffer from file or use preloaded slice
    // - call omer::itch::parse(buffer) or process(buffer, &mut noop_handler)
    // - report iterations * buffer_msg_count / time = msg/s
    c.bench_function("itch_parse_throughput", |b| {
        b.iter(|| {
            black_box(0u64);
        });
    });
}

criterion_group!(benches, itch_parse_throughput);
criterion_main!(benches);
