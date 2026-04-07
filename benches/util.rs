//! Minimal **no-harness** engine for benches that should compile with `default-features = false`.

use omer::book::service::BTreeOrderBook;
use omer::engine::OrderMatchingEngine;
use omer::events::NoOpEventSink;
use omer::matching::PriceCrossMatchingPolicy;
use omer::self_trade::AllowAllSelfTradePolicy;
use omer::sequence::CounterSequenceGenerator;
use omer::store::service::HashMapOrderStore;

pub type MinimalNoopEngine = OrderMatchingEngine<
    CounterSequenceGenerator,
    BTreeOrderBook,
    HashMapOrderStore,
    PriceCrossMatchingPolicy,
    AllowAllSelfTradePolicy,
    NoOpEventSink,
>;

pub fn minimal_noop_engine() -> MinimalNoopEngine {
    OrderMatchingEngine::new(
        CounterSequenceGenerator::new(),
        BTreeOrderBook::new(),
        HashMapOrderStore::new(),
        PriceCrossMatchingPolicy,
        AllowAllSelfTradePolicy,
        NoOpEventSink,
    )
}
