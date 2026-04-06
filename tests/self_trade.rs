//! VII. Self-Trade Prevention
//!
//! Detection and enforcement when same participant would match.

mod common;

use common::{EventSnapshot, add_cmd, engine_with_self_trade_rejection};
use omer::engine::OrderMatchingService;
use omer::error::Error;
use omer::types::{OrderType, Side, TimeInForce};

/// VII.1 Detection: given an incoming order and a resting order belonging to the same participant,
/// when they would otherwise match, then self-trade prevention must deterministically detect the violation.
#[test]
fn self_trade_detection() {
    let (mut engine, sink) = engine_with_self_trade_rejection();
    let sell = add_cmd(
        1,
        100,
        Side::Sell,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(sell).unwrap();
    sink.clear();
    let buy = add_cmd(
        2,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    let res = engine.add(buy);
    assert!(res.is_err(), "self-trade must be detected and rejected");
    assert!(matches!(res.unwrap_err(), Error::Rejection(_)));
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0].event, EventSnapshot::Rejected(_)));
}

/// VII.2 Enforcement: given self-trade prevention is enabled, when a violation occurs,
/// then no trade must occur (no Trade event emitted).
#[test]
fn self_trade_no_trade_emitted() {
    let (mut engine, sink) = engine_with_self_trade_rejection();
    let sell = add_cmd(
        1,
        100,
        Side::Sell,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(sell).unwrap();
    sink.clear();
    let buy = add_cmd(
        2,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        5,
        TimeInForce::Gtc,
    );
    let _ = engine.add(buy);
    let events = sink.snapshot();
    let any_trade = events
        .iter()
        .any(|e| matches!(&e.event, EventSnapshot::Trade { .. }));
    assert!(
        !any_trade,
        "self-trade violation must not produce any Trade event"
    );
}
