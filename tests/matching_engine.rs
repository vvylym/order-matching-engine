//! Integration checks for batch processing behavior on `OrderMatchingEngine`.

use omer::book::service::BTreeOrderBook;
use omer::engine::{AddOrderCommand, OrderCommand, OrderMatchingEngine};
use omer::events::NoOpEventSink;
use omer::matching::PriceCrossMatchingPolicy;
use omer::self_trade::AllowAllSelfTradePolicy;
use omer::sequence::CounterSequenceGenerator;
use omer::store::service::HashMapOrderStore;
use omer::types::{OrderType, Side, TimeInForce};

type TestEngine = OrderMatchingEngine<
    CounterSequenceGenerator,
    BTreeOrderBook,
    HashMapOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    NoOpEventSink,
>;

fn new_engine() -> TestEngine {
    OrderMatchingEngine::new(
        CounterSequenceGenerator::new(),
        BTreeOrderBook::new(),
        HashMapOrderStore::new(),
        PriceCrossMatchingPolicy,
        AllowAllSelfTradePolicy,
        NoOpEventSink,
    )
}

fn add_limit(id: u64, side: Side, price: i64, qty: i64) -> AddOrderCommand {
    AddOrderCommand {
        id,
        participant_id: 100,
        symbol_id: 1,
        side,
        order_type: OrderType::Limit,
        price: Some(price),
        quantity: qty,
        time_in_force: TimeInForce::Gtc,
        stop_price: None,
        max_visible_quantity: None,
        slippage: None,
        trailing_distance: None,
        trailing_step: None,
    }
}

#[test]
fn process_batch_stops_on_first_error_and_keeps_prior_commits() {
    let mut engine = new_engine();
    let cmds = vec![
        OrderCommand::Add(add_limit(1, Side::Buy, 100, 10)),
        // Invalid: market order with an explicit price should be rejected.
        OrderCommand::Add(AddOrderCommand {
            id: 2,
            participant_id: 100,
            symbol_id: 1,
            side: Side::Buy,
            order_type: OrderType::Market,
            price: Some(101),
            quantity: 1,
            time_in_force: TimeInForce::Ioc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
        }),
        OrderCommand::Add(add_limit(3, Side::Buy, 99, 5)),
    ];

    assert!(engine.process_batch(cmds).is_err());
    assert_eq!(engine.best_bid(), Some(100));
    assert_eq!(engine.best_ask(), None);
}

#[test]
fn process_batch_applies_mixed_commands_in_order() {
    let mut engine = new_engine();
    let cmds = vec![
        OrderCommand::Add(add_limit(10, Side::Buy, 101, 4)),
        OrderCommand::CancelByOrderId(omer::engine::CancelByOrderIdCommand {
            order_id: 10,
        }),
        OrderCommand::Add(add_limit(11, Side::Sell, 105, 2)),
    ];

    assert!(engine.process_batch(cmds).is_ok());
    assert_eq!(engine.best_bid(), None);
    assert_eq!(engine.best_ask(), Some(105));
}
