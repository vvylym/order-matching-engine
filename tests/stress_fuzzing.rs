//! XI. Stress & Adversarial Scenarios
//!
//! High churn; property-based fuzzing with quickcheck.

mod common;

use common::{add_cmd, engine_with_memory, engine_with_shared_state};
use omer::book::PriceBook;
use omer::engine::{CancelOrderCommand, OrderCommand, OrderMatchingService};
use omer::types::{OrderType, Side, TimeInForce};
use quickcheck::QuickCheck;
use quickcheck_macros::quickcheck;

/// One fuzz operation: (is_add, side_u, tif_u, qty_raw) to reduce type complexity.
type FuzzOp = (bool, u8, u8, i64);

/// Build a deterministic sequence of commands from fuzz ops (same ops => same commands).
fn build_commands(ops: &[FuzzOp]) -> Vec<OrderCommand> {
    let mut next_id = 1u64;
    let mut live_ids: Vec<u64> = Vec::new();
    let mut seq_by_id: std::collections::HashMap<u64, u64> =
        std::collections::HashMap::new();
    let mut out = Vec::with_capacity(ops.len().min(100));
    for (is_add, side_u, tif_u, qty_raw) in ops.iter().take(100) {
        let side = if *side_u % 2 == 0 {
            Side::Buy
        } else {
            Side::Sell
        };
        let tif = if *tif_u % 2 == 0 {
            TimeInForce::Gtc
        } else {
            TimeInForce::Ioc
        };
        let qty = qty_raw.unsigned_abs() as i64;
        let qty = if qty == 0 { 1 } else { qty.min(1000) };
        if *is_add {
            let seq = next_id.saturating_sub(1);
            seq_by_id.insert(next_id, seq);
            let cmd = OrderCommand::Add(add_cmd(
                next_id,
                100,
                side,
                OrderType::Limit,
                Some(50),
                qty,
                tif,
            ));
            out.push(cmd);
            live_ids.push(next_id);
            next_id += 1;
        } else if !live_ids.is_empty() {
            let idx = (*side_u as usize) % live_ids.len();
            let order_id = live_ids[idx];
            let seq = seq_by_id.get(&order_id).copied().unwrap_or(0);
            out.push(OrderCommand::Cancel(CancelOrderCommand {
                order_id,
                participant_id: 100,
                sequence: seq,
            }));
            live_ids.remove(idx);
        }
    }
    out
}

/// XI.2 Property: any valid command sequence runs without panic.
#[quickcheck]
fn prop_no_panic(ops: Vec<FuzzOp>) -> bool {
    let (mut engine, _sink) = engine_with_memory();
    for cmd in build_commands(&ops) {
        let _ = engine.process(cmd);
    }
    true
}

/// XI.2 Property: same command sequence yields same event count (determinism).
#[quickcheck]
fn prop_determinism(ops: Vec<FuzzOp>) -> bool {
    let cmds = build_commands(&ops);
    let (mut e1, s1) = engine_with_memory();
    let (mut e2, s2) = engine_with_memory();
    for cmd in &cmds {
        let r1 = e1.process(cmd.clone());
        let r2 = e2.process(cmd.clone());
        if r1.is_ok() != r2.is_ok() {
            return false;
        }
    }
    s1.snapshot().len() == s2.snapshot().len()
}

/// XI.2 Property: after any command sequence, book is never crossed (best_bid <= best_ask).
#[quickcheck]
fn prop_book_never_crossed(ops: Vec<FuzzOp>) -> bool {
    let (mut engine, _sink, book_handle, _store) = engine_with_shared_state();
    for cmd in build_commands(&ops) {
        let _ = engine.process(cmd);
    }
    let book = book_handle.borrow();
    match (book.best_bid(), book.best_ask()) {
        (Some(bid), Some(ask)) => bid <= ask,
        _ => true,
    }
}

/// Explicit use of quickcheck crate so machete sees the dev-dependency (macro use is not visible).
#[test]
fn quickcheck_crate_used() {
    let _ = QuickCheck::new();
}

/// XI.1 High churn: many add/cancel without panic.
#[test]
fn high_churn_no_panic() {
    let (mut engine, _sink) = engine_with_memory();
    for i in 1..=50 {
        let add = OrderCommand::Add(add_cmd(
            i,
            100,
            Side::Buy,
            OrderType::Limit,
            Some(50 + (i % 3) as i64),
            10,
            TimeInForce::Gtc,
        ));
        let _ = engine.process(add);
    }
    for i in (1..=50).rev() {
        let cancel = OrderCommand::Cancel(CancelOrderCommand {
            order_id: i,
            participant_id: 100,
            sequence: i.saturating_sub(1),
        });
        let _ = engine.process(cancel);
    }
}
