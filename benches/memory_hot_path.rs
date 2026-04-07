//! Reports **allocation counts** (`allocation-counter`) for add + cancel-by-id on a stable id after a warm-up pass.

mod util;

use std::hint::black_box;

use allocation_counter::{AllocationInfo, measure};
use criterion::{Criterion, criterion_group, criterion_main};
use omer::engine::{
    AddOrderCommand, CancelByOrderIdCommand, OrderMatchingService,
};
use omer::types::{OrderType, Side, TimeInForce};

use util::minimal_noop_engine;

fn lim_buy(id: u64, price: i64) -> AddOrderCommand {
    AddOrderCommand {
        id,
        participant_id: 100,
        symbol_id: 1,
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(price),
        quantity: 1,
        time_in_force: TimeInForce::Gtc,
        stop_price: None,
        max_visible_quantity: None,
        slippage: None,
        trailing_distance: None,
        trailing_step: None,
    }
}

fn memory_hot_path(c: &mut Criterion) {
    {
        let mut eng = minimal_noop_engine();
        for i in 1..=8192_u64 {
            eng.add(lim_buy(i, 100 + (i as i64 % 200))).unwrap();
        }
        for i in 1..=8192_u64 {
            let _ =
                eng.cancel_by_order_id(CancelByOrderIdCommand { order_id: i });
        }
    }

    let mut eng = minimal_noop_engine();
    let mut group = c.benchmark_group("memory_hot_path_minimal_engine");
    group.bench_function("allocation_count_add_cancel_stable_id", |b| {
        b.iter(|| {
            let info: AllocationInfo = measure(|| {
                eng.add(lim_buy(1, 100)).unwrap();
                eng.cancel_by_order_id(CancelByOrderIdCommand { order_id: 1 })
                    .unwrap();
            });
            black_box(info.count_total);
        });
    });
    group.finish();
}

criterion_group!(benches, memory_hot_path);
criterion_main!(benches);
