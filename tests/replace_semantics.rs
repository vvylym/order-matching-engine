//! VI. Replace (Cancel-Replace) Semantics
//!
//! Atomicity, quantity reduction, immediate re-matching when price becomes marketable.

mod common;

use common::{EventSnapshot, add_cmd, engine_with_memory};
use omer::engine::{OrderCommand, OrderMatchingService, ReplaceOrderCommand};
use omer::error::Error;
use omer::types::{OrderType, Side, TimeInForce};

/// VI.1 Replace is atomic: no intermediate state where order is partially visible.
#[test]
fn replace_atomic() {
    let (mut engine, sink) = engine_with_memory();
    let add = add_cmd(
        1,
        100,
        Side::Sell,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(add).unwrap();
    sink.clear();
    let replace = ReplaceOrderCommand {
        order_id: 1,
        participant_id: 100,
        new_price: Some(49),
        new_quantity: 5,
        sequence: 0,
    };
    engine.replace(replace).unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 2); // Canceled(1), Accepted(1)
    assert!(matches!(events[0].event, EventSnapshot::Canceled(1)));
    assert!(matches!(events[1].event, EventSnapshot::Accepted(1)));
}

/// Cover process(OrderCommand::Replace) dispatch in engine trait.
#[test]
fn replace_via_process() {
    let (mut engine, sink) = engine_with_memory();
    let add = add_cmd(
        1,
        100,
        Side::Sell,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.process(OrderCommand::Add(add)).unwrap();
    sink.clear();
    let replace = ReplaceOrderCommand {
        order_id: 1,
        participant_id: 100,
        new_price: Some(49),
        new_quantity: 5,
        sequence: 0,
    };
    engine.process(OrderCommand::Replace(replace)).unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0].event, EventSnapshot::Canceled(1)));
    assert!(matches!(events[1].event, EventSnapshot::Accepted(1)));
}

/// Replace with new_quantity 0: InvalidQuantity rejected.
#[test]
fn replace_invalid_quantity_rejected() {
    let (mut engine, sink) = engine_with_memory();
    let add = add_cmd(
        1,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(add).unwrap();
    sink.clear();
    let replace = ReplaceOrderCommand {
        order_id: 1,
        participant_id: 100,
        new_price: Some(50),
        new_quantity: 0,
        sequence: 0,
    };
    let res = engine.replace(replace);
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), Error::Rejection(_)));
    assert!(matches!(
        &sink.snapshot()[0].event,
        EventSnapshot::Rejected(_)
    ));
}

/// Replace limit order with new_price None: InvalidPrice rejected.
#[test]
fn replace_invalid_price_limit_to_none_rejected() {
    let (mut engine, sink) = engine_with_memory();
    let add = add_cmd(
        1,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(add).unwrap();
    sink.clear();
    let replace = ReplaceOrderCommand {
        order_id: 1,
        participant_id: 100,
        new_price: None, // invalid for limit order
        new_quantity: 10,
        sequence: 0,
    };
    let res = engine.replace(replace);
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), Error::Rejection(_)));
}

/// Replace with wrong participant: ParticipantMismatch rejected.
#[test]
fn replace_wrong_participant_rejected() {
    let (mut engine, sink) = engine_with_memory();
    let add = add_cmd(
        1,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(add).unwrap();
    sink.clear();
    let replace = ReplaceOrderCommand {
        order_id: 1,
        participant_id: 999,
        new_price: Some(50),
        new_quantity: 5,
        sequence: 0,
    };
    let res = engine.replace(replace);
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), Error::Rejection(_)));
}

/// Replace with wrong sequence: StaleSequence rejected.
#[test]
fn replace_wrong_sequence_rejected() {
    let (mut engine, sink) = engine_with_memory();
    let add = add_cmd(
        1,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(add).unwrap();
    sink.clear();
    let replace = ReplaceOrderCommand {
        order_id: 1,
        participant_id: 100,
        new_price: Some(50),
        new_quantity: 5,
        sequence: 99, // wrong; order has sequence 0
    };
    let res = engine.replace(replace);
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), Error::Rejection(_)));
}

/// VI.2 Quantity reduction: remaining quantity persists with correct priority.
#[test]
fn replace_quantity_reduction() {
    let (mut engine, sink) = engine_with_memory();
    let add = add_cmd(
        1,
        100,
        Side::Sell,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(add).unwrap();
    sink.clear();
    let replace = ReplaceOrderCommand {
        order_id: 1,
        participant_id: 100,
        new_price: Some(50),
        new_quantity: 4,
        sequence: 0,
    };
    engine.replace(replace).unwrap();
    sink.clear();
    let buy = add_cmd(
        2,
        101,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(buy).unwrap();
    let events = sink.snapshot();
    assert!(!events.is_empty());
    match &events[0].event {
        EventSnapshot::Trade { quantity, .. } => assert_eq!(*quantity, 4),
        _ => panic!("expected first event to be trade with qty 4"),
    }
}

/// VI.3 Replace to marketable price: order must enter matching immediately (spec).
#[test]
fn replace_to_marketable_rematches() {
    let (mut engine, sink) = engine_with_memory();
    let sell = add_cmd(
        2,
        101,
        Side::Sell,
        OrderType::Limit,
        Some(50),
        5,
        TimeInForce::Gtc,
    );
    engine.add(sell).unwrap();
    let buy = add_cmd(
        1,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(49),
        10,
        TimeInForce::Gtc,
    );
    engine.add(buy).unwrap();
    sink.clear();
    let replace = ReplaceOrderCommand {
        order_id: 1,
        participant_id: 100,
        new_price: Some(50),
        new_quantity: 10,
        sequence: 1,
    };
    engine.replace(replace).unwrap();
    let events = sink.snapshot();
    let has_trade = events
        .iter()
        .any(|e| matches!(&e.event, EventSnapshot::Trade { .. }));
    assert!(has_trade, "replace to marketable should produce trade");
}
