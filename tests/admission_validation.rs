//! I. Order Admission & Validation
//!
//! Given/When/Then tests for valid acceptance, invalid price/quantity,
//! and duplicate order identifier.

mod common;

use common::{EventSnapshot, add_cmd, engine_with_memory};
use omer::engine::OrderMatchingService;
use omer::error::Error;
use omer::types::{OrderType, Side, TimeInForce};
use rstest::rstest;

/// I.1 Valid order acceptance: well-formed order is accepted (parameterized over side and type).
#[rstest]
#[case(Side::Buy, OrderType::Limit, Some(50))]
#[case(Side::Sell, OrderType::Limit, Some(50))]
#[case(Side::Buy, OrderType::Market, None)]
#[case(Side::Sell, OrderType::Market, None)]
fn valid_order_accepted(
    #[case] side: Side,
    #[case] order_type: OrderType,
    #[case] price: Option<i64>,
) {
    let (mut engine, sink) = engine_with_memory();
    let cmd = add_cmd(1, 100, side, order_type, price, 10, TimeInForce::Gtc);
    let res = engine.add(cmd);
    assert!(res.is_ok());
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event, EventSnapshot::Accepted(1)));
}

/// I.2 Invalid price: limit with None or market with Some is rejected.
#[rstest]
#[case(OrderType::Limit, None, "limit requires price")]
#[case(OrderType::Market, Some(50), "market must have no price")]
fn invalid_price_rejected(
    #[case] order_type: OrderType,
    #[case] price: Option<i64>,
    #[case] _reason: &str,
) {
    let (mut engine, sink) = engine_with_memory();
    let cmd = add_cmd(1, 100, Side::Buy, order_type, price, 10, TimeInForce::Gtc);
    let res = engine.add(cmd);
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), Error::Rejection(_)));
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0].event, EventSnapshot::Rejected(_)));
}

/// I.3 Invalid quantity: zero or negative quantity is rejected.
#[rstest]
#[case(0)]
#[case(-1)]
#[case(-100)]
fn invalid_quantity_rejected(#[case] qty: i64) {
    let (mut engine, sink) = engine_with_memory();
    let cmd = add_cmd(
        1,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        qty,
        TimeInForce::Gtc,
    );
    let res = engine.add(cmd);
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), Error::Rejection(_)));
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0].event, EventSnapshot::Rejected(_)));
}

/// I.4 Duplicate order identifier: when engine checks for existing id, duplicate is rejected.
#[test]
fn duplicate_order_id_spec() {
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
    engine.add(cmd.clone()).unwrap();
    sink.clear();
    let res = engine.add(cmd);
    // Engine must not accept duplicate: either Rejection or OrderStore::AlreadyExists.
    assert!(
        res.is_err(),
        "duplicate order id must be rejected or fail at store"
    );
    let err = res.unwrap_err();
    assert!(
        matches!(err, Error::Rejection(_)) || matches!(err, Error::OrderStore(_)),
        "expected Rejection or OrderStore error, got {:?}",
        err
    );
}
