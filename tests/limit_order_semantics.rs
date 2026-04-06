//! II. Limit Order Semantics
//!
//! Non-marketable vs marketable limit orders, partial fill with remainder, full fill.

mod common;

use common::{EventSnapshot, add_cmd, engine_with_memory};
use omer::engine::OrderMatchingService;
use omer::types::{OrderType, Side, TimeInForce};
use rstest::rstest;

/// II.1 Non-marketable limit order: rests in book (parameterized over side).
#[rstest]
#[case(Side::Buy)]
#[case(Side::Sell)]
fn non_marketable_limit_rests_in_book(#[case] side: Side) {
    let (mut engine, sink) = engine_with_memory();
    let cmd = add_cmd(
        1,
        100,
        side,
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

/// II.1b Non-marketable limit does not match: can_match false, order pushed back and incoming rests.
#[test]
fn non_marketable_does_not_match_incoming_rests() {
    let (mut engine, sink) = engine_with_memory();
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
        101,
        Side::Buy,
        OrderType::Limit,
        Some(49), // below best ask 50: does not cross
        10,
        TimeInForce::Gtc,
    );
    engine.add(buy).unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event, EventSnapshot::Accepted(2)));
}

/// II.2 Marketable limit order: matches immediately (parameterized: resting side / aggressor side).
#[rstest]
#[case(Side::Sell, Side::Buy, 50_i64)] // resting sell @ 50, buy @ 50
#[case(Side::Buy, Side::Sell, 50_i64)] // resting buy @ 50, sell @ 50
fn marketable_limit_matches_immediately(
    #[case] resting_side: Side,
    #[case] aggressor_side: Side,
    #[case] price: i64,
) {
    let (mut engine, sink) = engine_with_memory();
    let resting = add_cmd(
        1,
        100,
        resting_side,
        OrderType::Limit,
        Some(price),
        10,
        TimeInForce::Gtc,
    );
    engine.add(resting).unwrap();
    sink.clear();
    let agg = add_cmd(
        2,
        101,
        aggressor_side,
        OrderType::Limit,
        Some(price),
        10,
        TimeInForce::Gtc,
    );
    engine.add(agg).unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event, EventSnapshot::Trade { .. }));
}

/// II.3 Partial fill with remainder: filled quantity traded, remainder rests.
#[test]
fn partial_fill_remainder_rests() {
    let (mut engine, sink) = engine_with_memory();
    let sell = add_cmd(
        1,
        100,
        Side::Sell,
        OrderType::Limit,
        Some(50),
        5,
        TimeInForce::Gtc,
    );
    engine.add(sell).unwrap();
    sink.clear();
    // Buy 10: 5 match, 5 rest
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
    assert_eq!(events.len(), 2); // Trade then Accepted
    assert!(matches!(
        events[0].event,
        EventSnapshot::Trade { quantity: 5, .. }
    ));
    assert!(matches!(events[1].event, EventSnapshot::Accepted(2)));
}

/// II.4 Full fill: order never rests (parameterized over resting/aggressor side).
#[rstest]
#[case(Side::Sell, Side::Buy)]
#[case(Side::Buy, Side::Sell)]
fn full_fill_does_not_rest(
    #[case] resting_side: Side,
    #[case] aggressor_side: Side,
) {
    let (mut engine, sink) = engine_with_memory();
    let resting = add_cmd(
        1,
        100,
        resting_side,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(resting).unwrap();
    sink.clear();
    let agg = add_cmd(
        2,
        101,
        aggressor_side,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(agg).unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event, EventSnapshot::Trade { .. }));
}
