//! V. Cancel Semantics
//!
//! Valid cancel, cancel of unknown order, ownership enforcement.

mod common;

use common::{EventSnapshot, add_cmd, engine_with_memory};
use omer::engine::{CancelOrderCommand, OrderMatchingService};
use omer::error::Error;
use omer::types::{OrderType, Side, TimeInForce};
use rstest::rstest;

/// V.1 Valid cancel: resting order is removed; Canceled emitted (parameterized over side).
#[rstest]
#[case(Side::Buy)]
#[case(Side::Sell)]
fn valid_cancel_removes_order(#[case] side: Side) {
    let (mut engine, sink) = engine_with_memory();
    let add = add_cmd(
        1,
        100,
        side,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(add).unwrap();
    sink.clear();
    let cancel = CancelOrderCommand {
        order_id: 1,
        participant_id: 100,
        sequence: 0,
    };
    let res = engine.cancel(cancel);
    assert!(res.is_ok());
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event, EventSnapshot::Canceled(1)));
}

/// V.2 Cancel of unknown order: rejected or no-op, no state mutation.
#[test]
fn cancel_unknown_order_rejected() {
    let (mut engine, sink) = engine_with_memory();
    let cancel = CancelOrderCommand {
        order_id: 999,
        participant_id: 100,
        sequence: 0,
    };
    let res = engine.cancel(cancel);
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), Error::Rejection(_)));
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0].event, EventSnapshot::Rejected(_)));
}

/// V.2b Cancel with wrong sequence: StaleSequence rejected.
#[test]
fn cancel_wrong_sequence_rejected() {
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
    let cancel = CancelOrderCommand {
        order_id: 1,
        participant_id: 100,
        sequence: 99, // wrong; order has sequence 0
    };
    let res = engine.cancel(cancel);
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), Error::Rejection(_)));
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0].event, EventSnapshot::Rejected(_)));
}

/// V.3 Ownership: cancel by wrong participant is rejected.
#[test]
fn cancel_wrong_participant_rejected() {
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
    let cancel = CancelOrderCommand {
        order_id: 1,
        participant_id: 999, // not owner
        sequence: 0,
    };
    let res = engine.cancel(cancel);
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), Error::Rejection(_)));
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0].event, EventSnapshot::Rejected(_)));
}
