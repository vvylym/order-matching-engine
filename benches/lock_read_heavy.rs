//! Read-heavy lock comparison for shared state wrappers.
//!
//! Compares:
//! - `Arc<std::sync::RwLock<T>>`
//! - `Arc<parking_lot::RwLock<T>>`
//! - `Arc<tokio::sync::RwLock<T>>`

use std::collections::HashMap;
use std::hint::black_box;
use std::sync::{Arc, RwLock};

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use parking_lot::RwLock as ParkingRwLock;
use tokio::runtime::Runtime;
use tokio::sync::RwLock as TokioRwLock;

const OPS: u64 = 200_000;
const WRITE_EVERY: u64 = 20;

fn lock_read_heavy(c: &mut Criterion) {
    let mut group = c.benchmark_group("lock_read_heavy_shared_map");
    group.throughput(Throughput::Elements(OPS));

    group.bench_function("std_rwlock_arc", |b| {
        b.iter(|| {
            let shared = Arc::new(RwLock::new(HashMap::<u64, u64>::new()));
            for i in 0..OPS {
                if i % WRITE_EVERY == 0 {
                    let mut w = shared.write().expect("rwlock poisoned");
                    w.insert(i, i);
                } else {
                    let r = shared.read().expect("rwlock poisoned");
                    black_box(r.get(&(i - (i % WRITE_EVERY))).copied());
                }
            }
        });
    });

    group.bench_function("parking_lot_rwlock_arc", |b| {
        b.iter(|| {
            let shared = Arc::new(ParkingRwLock::new(HashMap::<u64, u64>::new()));
            for i in 0..OPS {
                if i % WRITE_EVERY == 0 {
                    let mut w = shared.write();
                    w.insert(i, i);
                } else {
                    let r = shared.read();
                    black_box(r.get(&(i - (i % WRITE_EVERY))).copied());
                }
            }
        });
    });

    let rt = Runtime::new().expect("tokio runtime");
    group.bench_function("tokio_rwlock_arc", |b| {
        b.iter(|| {
            let shared = Arc::new(TokioRwLock::new(HashMap::<u64, u64>::new()));
            rt.block_on(async {
                for i in 0..OPS {
                    if i % WRITE_EVERY == 0 {
                        let mut w = shared.write().await;
                        w.insert(i, i);
                    } else {
                        let r = shared.read().await;
                        black_box(r.get(&(i - (i % WRITE_EVERY))).copied());
                    }
                }
            });
        });
    });

    group.finish();
}

criterion_group!(benches, lock_read_heavy);
criterion_main!(benches);
