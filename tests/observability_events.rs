//! X. Observability & Event Emission
//!
//! One event per outcome; events in causal order.

mod common;

use common::{EventSnapshot, add_cmd, engine_with_memory};
use omer::engine::OrderMatchingService;
use omer::types::{OrderType, Side, TimeInForce};

/// X.1 Event completeness: accepted -> Accepted, rejected -> Rejected, canceled -> Canceled, matched -> Trade.
#[test]
fn accepted_order_emits_accepted() {
    let (mut engine, sink) = engine_with_memory();
    let cmd = add_cmd(
        1,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(cmd).unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event, EventSnapshot::Accepted(1)));
}

#[test]
fn trade_emits_trade_event() {
    let (mut engine, sink) = engine_with_memory();
    engine
        .add(add_cmd(
            1,
            100,
            Side::Sell,
            OrderType::Limit,
            Some(50),
            10,
            TimeInForce::Gtc,
        ))
        .unwrap();
    sink.clear();
    engine
        .add(add_cmd(
            2,
            101,
            Side::Buy,
            OrderType::Limit,
            Some(50),
            10,
            TimeInForce::Gtc,
        ))
        .unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event, EventSnapshot::Trade { .. }));
}

#[test]
fn cancel_emits_canceled() {
    let (mut engine, sink) = engine_with_memory();
    engine
        .add(add_cmd(
            1,
            100,
            Side::Buy,
            OrderType::Limit,
            Some(50),
            10,
            TimeInForce::Gtc,
        ))
        .unwrap();
    sink.clear();
    engine
        .cancel(omer::engine::CancelOrderCommand {
            order_id: 1,
            participant_id: 100,
            sequence: 0,
        })
        .unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event, EventSnapshot::Canceled(1)));
}

/// X.1 (rejected): given a rejected order (e.g. invalid quantity), when processed, then exactly one Rejected event is emitted.
#[test]
fn rejected_order_emits_rejected() {
    let (mut engine, sink) = engine_with_memory();
    let invalid_qty = add_cmd(
        1,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        0,
        TimeInForce::Gtc,
    );
    let _ = engine.add(invalid_qty);
    let events = sink.snapshot();
    assert_eq!(events.len(), 1, "exactly one event for rejected order");
    assert!(matches!(&events[0].event, EventSnapshot::Rejected(_)));
}

/// X.2 Event ordering: multiple commands produce events in execution order.
#[test]
fn events_in_causal_order() {
    let (mut engine, sink) = engine_with_memory();
    engine
        .add(add_cmd(
            1,
            100,
            Side::Sell,
            OrderType::Limit,
            Some(50),
            5,
            TimeInForce::Gtc,
        ))
        .unwrap();
    engine
        .add(add_cmd(
            2,
            101,
            Side::Buy,
            OrderType::Limit,
            Some(50),
            10,
            TimeInForce::Gtc,
        ))
        .unwrap();
    let events = sink.snapshot();
    assert!(events.len() >= 2);
    assert!(matches!(events[0].event, EventSnapshot::Accepted(1)));
    assert!(matches!(events[1].event, EventSnapshot::Trade { .. }));
    assert!(matches!(events[2].event, EventSnapshot::Accepted(2)));
}
