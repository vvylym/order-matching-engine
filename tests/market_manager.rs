//! Integration checks for ITCH-style command variants used by higher-level market orchestration.

use omer::book::service::BTreeOrderBook;
use omer::engine::{
    AddOrderCommand, ExecuteOrderCommand, OrderCommand, OrderMatchingEngine,
    OrderMatchingService, ReduceOrderCommand, ReplaceOrderByNewIdCommand,
};
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

#[test]
fn itch_style_reduce_execute_and_replace_by_new_id_flow() {
    let mut engine = new_engine();

    let add = AddOrderCommand {
        id: 10,
        participant_id: 100,
        symbol_id: 1,
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(101),
        quantity: 10,
        time_in_force: TimeInForce::Gtc,
        stop_price: None,
        max_visible_quantity: None,
        slippage: None,
        trailing_distance: None,
        trailing_step: None,
    };

    assert!(engine.process(OrderCommand::Add(add)).is_ok());
    assert_eq!(engine.best_bid(), Some(101));

    assert!(
        engine
            .process(OrderCommand::Reduce(ReduceOrderCommand {
                order_id: 10,
                quantity: 3,
            }))
            .is_ok()
    );
    assert_eq!(engine.best_bid(), Some(101));

    assert!(
        engine
            .process(OrderCommand::ReplaceByNewId(ReplaceOrderByNewIdCommand {
                old_order_id: 10,
                new_order_id: 11,
                new_price: 102,
                new_quantity: 4,
                symbol_id: Some(1),
                side: Some(Side::Buy),
            }))
            .is_ok()
    );
    assert_eq!(engine.best_bid(), Some(102));

    assert!(
        engine
            .process(OrderCommand::Execute(ExecuteOrderCommand {
                order_id: 11,
                quantity: 4,
            }))
            .is_ok()
    );
    assert_eq!(engine.best_bid(), None);
    assert_eq!(engine.best_ask(), None);
}
