//! IV. Price–Time Priority
//!
//! FIFO at same price, priority across price levels, priority loss on replace.

mod common;

use common::{EventSnapshot, add_cmd, engine_with_memory};
use omer::engine::OrderMatchingService;
use omer::types::{OrderType, Side, TimeInForce};
use rstest::rstest;

/// IV.1 FIFO at same price: fills occur in insertion order (parameterized: two resting, one aggressor).
#[rstest]
#[case(Side::Sell, Side::Buy)] // two sells @ 50, one buy fills 1 then 2
#[case(Side::Buy, Side::Sell)] // two buys @ 50, one sell fills 1 then 2
fn fifo_at_same_price(#[case] resting_side: Side, #[case] aggressor_side: Side) {
    let (mut engine, sink) = engine_with_memory();
    let r1 = add_cmd(
        1,
        100,
        resting_side,
        OrderType::Limit,
        Some(50),
        5,
        TimeInForce::Gtc,
    );
    let r2 = add_cmd(
        2,
        100,
        resting_side,
        OrderType::Limit,
        Some(50),
        5,
        TimeInForce::Gtc,
    );
    engine.add(r1).unwrap();
    engine.add(r2).unwrap();
    sink.clear();
    let agg = add_cmd(
        3,
        101,
        aggressor_side,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(agg).unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 2);
    let trade1 = match &events[0].event {
        EventSnapshot::Trade { resting, .. } => *resting,
        _ => panic!("expected trade"),
    };
    let trade2 = match &events[1].event {
        EventSnapshot::Trade { resting, .. } => *resting,
        _ => panic!("expected trade"),
    };
    assert_eq!(trade1, 1, "first fill must be order 1 (FIFO)");
    assert_eq!(trade2, 2, "second fill must be order 2 (FIFO)");
}

/// IV.2 Priority across price levels: best price matched first.
#[test]
fn priority_across_price_levels() {
    let (mut engine, sink) = engine_with_memory();
    let s_51 = add_cmd(
        1,
        100,
        Side::Sell,
        OrderType::Limit,
        Some(51),
        5,
        TimeInForce::Gtc,
    );
    let s_50 = add_cmd(
        2,
        100,
        Side::Sell,
        OrderType::Limit,
        Some(50),
        5,
        TimeInForce::Gtc,
    );
    engine.add(s_51).unwrap();
    engine.add(s_50).unwrap();
    sink.clear();
    let buy = add_cmd(
        3,
        101,
        Side::Buy,
        OrderType::Limit,
        Some(51),
        5,
        TimeInForce::Gtc,
    );
    engine.add(buy).unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    // Best ask is 50; we should match at 50 first (order 2)
    match &events[0].event {
        EventSnapshot::Trade { resting, .. } => assert_eq!(*resting, 2),
        _ => panic!("expected trade"),
    }
}

/// IV.3 Replace: order loses time priority when reinserted (spec: must go to back of queue).
#[test]
fn replace_loses_time_priority() {
    let (mut engine, sink) = engine_with_memory();
    let add1 = add_cmd(
        1,
        100,
        Side::Sell,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(add1).unwrap();
    let seq = 0; // first order got sequence 0
    let replace = omer::engine::ReplaceOrderCommand {
        order_id: 1,
        participant_id: 100,
        new_price: Some(50),
        new_quantity: 15,
        sequence: seq,
    };
    engine.replace(replace).unwrap();
    sink.clear();
    // Add another sell at 50 (order 2), then buy 5: should fill order 2 first (FIFO after replace).
    let add2 = add_cmd(
        2,
        100,
        Side::Sell,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(add2).unwrap();
    let buy = add_cmd(
        3,
        101,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        5,
        TimeInForce::Gtc,
    );
    engine.add(buy).unwrap();
    let events = sink.snapshot();
    let trade = events
        .iter()
        .find(|e| matches!(&e.event, EventSnapshot::Trade { .. }))
        .expect("expected trade");
    match &trade.event {
        EventSnapshot::Trade { resting, .. } => {
            assert_eq!(*resting, 2, "replaced order 1 loses priority to 2")
        }
        _ => unreachable!(),
    }
}
