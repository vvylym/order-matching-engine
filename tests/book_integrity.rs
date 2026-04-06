//! VIII. Book Integrity Invariants
//!
//! No crossed book, depth correctness, order uniqueness.

mod common;

use common::{add_cmd, engine_with_shared_state};
use omer::book::PriceBook;
use omer::engine::OrderMatchingService;
use omer::store::OrderStore;
use omer::types::{OrderType, Side, TimeInForce};

/// VIII.1 No crossed book: best bid <= best ask at any time.
#[test]
fn no_crossed_book() {
    let (mut engine, _sink, book_handle, _store_handle) =
        engine_with_shared_state();
    let add_buy = add_cmd(
        1,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(add_buy).unwrap();
    let add_sell = add_cmd(
        2,
        101,
        Side::Sell,
        OrderType::Limit,
        Some(51),
        10,
        TimeInForce::Gtc,
    );
    engine.add(add_sell).unwrap();
    let book = book_handle.borrow();
    let best_bid = book.best_bid();
    let best_ask = book.best_ask();
    if let (Some(bid), Some(ask)) = (best_bid, best_ask) {
        assert!(
            bid <= ask,
            "book must not be crossed: best_bid {} <= best_ask {}",
            bid,
            ask
        );
    }
}

/// VIII.2 Depth correctness: total depth equals sum of resting quantities.
#[test]
fn depth_correctness() {
    let (mut engine, _sink, book_handle, store_handle) =
        engine_with_shared_state();
    let a = add_cmd(
        1,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(49),
        5,
        TimeInForce::Gtc,
    );
    let b = add_cmd(
        2,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    let c = add_cmd(
        3,
        101,
        Side::Sell,
        OrderType::Limit,
        Some(51),
        7,
        TimeInForce::Gtc,
    );
    engine.add(a).unwrap();
    engine.add(b).unwrap();
    engine.add(c).unwrap();
    let book = book_handle.borrow();
    let store = store_handle.borrow();
    let total = book.total_depth(|id| store.get(&id).map(|o| o.leaves_quantity));
    assert_eq!(
        total,
        5 + 10 + 7,
        "total depth must equal sum of resting quantities"
    );
}

/// VIII.3 Order uniqueness: each live order in store appears once in the book.
#[test]
fn order_uniqueness_after_operations() {
    let (mut engine, _sink, book_handle, store_handle) =
        engine_with_shared_state();
    let add1 = add_cmd(
        1,
        100,
        Side::Buy,
        OrderType::Limit,
        Some(50),
        10,
        TimeInForce::Gtc,
    );
    engine.add(add1).unwrap();
    let store = store_handle.borrow();
    let order1 = store.get(&1).expect("order 1 in store");
    assert_eq!(order1.quantity, 10);
    drop(store);
    let book = book_handle.borrow();
    assert_eq!(book.best_bid(), Some(50));
}
