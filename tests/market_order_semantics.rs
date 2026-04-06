//! III. Market Order Semantics
//!
//! Immediate execution, partial execution when insufficient liquidity, no resting.

mod common;

use common::{EventSnapshot, add_cmd, engine_with_memory};
use omer::engine::OrderMatchingService;
use omer::error::Error;
use omer::types::{OrderType, Side, TimeInForce};
use rstest::rstest;

/// III.1 Market order with available liquidity: immediately consumes (parameterized over aggressor side).
#[rstest]
#[case(Side::Buy)] // buy hits resting sell
#[case(Side::Sell)] // sell hits resting buy
fn market_order_immediate_execution(#[case] aggressor_side: Side) {
    let (mut engine, sink) = engine_with_memory();
    let resting_side = match aggressor_side {
        Side::Buy => Side::Sell,
        Side::Sell => Side::Buy,
    };
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
    let market = add_cmd(
        2,
        101,
        aggressor_side,
        OrderType::Market,
        None,
        10,
        TimeInForce::Ioc,
    );
    engine.add(market).unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event, EventSnapshot::Trade { .. }));
}

/// III.2 Partial execution: quantity exceeds opposing liquidity; remainder canceled (IOC).
#[test]
fn market_order_partial_fill_remainder_canceled() {
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
    let buy = add_cmd(
        2,
        101,
        Side::Buy,
        OrderType::Market,
        None,
        10,
        TimeInForce::Ioc,
    );
    let res = engine.add(buy);
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), Error::Rejection(_)));
    let events = sink.snapshot();
    assert!(
        !events.is_empty(),
        "partial fill must emit at least the trade"
    );
    assert!(matches!(
        events[0].event,
        EventSnapshot::Trade { quantity: 5, .. }
    ));
}

/// III.3 Market order never rests in the book (parameterized over side).
#[rstest]
#[case(Side::Buy)]
#[case(Side::Sell)]
fn market_order_never_rests(#[case] side: Side) {
    let (mut engine, sink) = engine_with_memory();
    let cmd =
        add_cmd(1, 100, side, OrderType::Market, None, 10, TimeInForce::Gtc);
    engine.add(cmd).unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event, EventSnapshot::Accepted(1)));
}
