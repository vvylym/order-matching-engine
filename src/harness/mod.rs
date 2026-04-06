#![allow(missing_docs)]
#![allow(clippy::type_complexity)]

//! Shared in-memory **store**, **matching / self-trade policies**, and **event sink** for tests and benches.
//! Swap only the **`PriceBook`** via [`engine_with_book`] or the convenience `engine_with_*` helpers
//! so latency numbers stay comparable.
//!
//! Enabled via the **`harness`** crate feature (on by default). Use `default-features = false` to omit.

mod memory;
mod policy;
mod sink;

pub use memory::{
    InMemoryOrderStore, InMemoryPriceBook, SharedOrderStore,
    SharedOrderStoreHandle, SharedPriceBook, SharedPriceBookHandle,
};
pub use policy::{
    AllowSelfTradePolicy, CrossingMatchingPolicy, RejectSelfTradePolicy,
};
pub use sink::CollectingEventSink;

use std::cell::RefCell;
use std::rc::Rc;

use crate::book::service::{BTreeOrderBook, DashSkipOrderBook, PoolLevelOrderBook};
use crate::book::PriceBook;
use crate::engine::OrderMatchingEngine;
use crate::events::Event;
use crate::sequence::SequenceGenerator;
use crate::types::{OrderId, Price, Quantity, Sequence, Side};

/// Engine wired with book type **`PB`**, in-memory store, harness policies, and event collector.
///
/// Use [`engine_with_book`], [`engine_with_memory`], [`engine_with_btree_book`], etc., to compare
/// book implementations under the same harness in benchmarks.
pub type EngineWithBook<PB> = OrderMatchingEngine<
    IncrementalSequence,
    PB,
    InMemoryOrderStore,
    CrossingMatchingPolicy,
    AllowSelfTradePolicy,
    CollectingEventSink,
>;

/// In-memory engine used by most **integration tests** (legacy default book).
pub type EngineWithMemory = EngineWithBook<InMemoryPriceBook>;

#[allow(dead_code)]
pub type EngineWithSelfTradeRejection = OrderMatchingEngine<
    IncrementalSequence,
    InMemoryPriceBook,
    InMemoryOrderStore,
    CrossingMatchingPolicy,
    RejectSelfTradePolicy,
    CollectingEventSink,
>;

#[allow(dead_code)]
pub type EngineWithSharedState = OrderMatchingEngine<
    IncrementalSequence,
    SharedPriceBookHandle,
    SharedOrderStoreHandle,
    CrossingMatchingPolicy,
    AllowSelfTradePolicy,
    CollectingEventSink,
>;

/// Deterministic sequence: monotonic `next`, `next_at_end_of_queue` for replace FIFO.
#[derive(Default)]
pub struct IncrementalSequence {
    pub next: Sequence,
    back_count: Sequence,
}

impl IncrementalSequence {
    #[allow(dead_code)]
    pub fn new(next: Sequence) -> Self {
        Self {
            next,
            back_count: 0,
        }
    }
}

impl SequenceGenerator for IncrementalSequence {
    fn next(
        &mut self,
    ) -> Result<Sequence, crate::sequence::SequenceGeneratorError> {
        let s = self.next;
        self.next = self.next.saturating_add(1);
        Ok(s)
    }

    fn next_at_end_of_queue(
        &mut self,
    ) -> Result<Sequence, crate::sequence::SequenceGeneratorError> {
        let c = self.back_count;
        self.back_count = self.back_count.saturating_add(1);
        Ok(Sequence::MAX.saturating_sub(1).saturating_sub(c))
    }
}

#[allow(dead_code)]
pub fn engine_with_self_trade_rejection()
-> (EngineWithSelfTradeRejection, CollectingEventSink) {
    let seq = IncrementalSequence::default();
    let book = InMemoryPriceBook::default();
    let store = InMemoryOrderStore::default();
    let matching = CrossingMatchingPolicy;
    let self_trade = RejectSelfTradePolicy;
    let sink = CollectingEventSink::default();
    let sink_clone = sink.clone();
    let engine =
        OrderMatchingEngine::new(seq, book, store, matching, self_trade, sink);
    (engine, sink_clone)
}

/// Build an engine with the given **`PriceBook`** implementation (same store, policies, sink).
pub fn engine_with_book<PB: PriceBook + Default>(
) -> (EngineWithBook<PB>, CollectingEventSink) {
    let seq = IncrementalSequence::default();
    let book = PB::default();
    let store = InMemoryOrderStore::default();
    let matching = CrossingMatchingPolicy;
    let self_trade = AllowSelfTradePolicy;
    let sink = CollectingEventSink::default();
    let sink_clone = sink.clone();
    let engine =
        OrderMatchingEngine::new(seq, book, store, matching, self_trade, sink);
    (engine, sink_clone)
}

pub fn engine_with_memory() -> (EngineWithMemory, CollectingEventSink) {
    engine_with_book::<InMemoryPriceBook>()
}

/// [`BTreeOrderBook`]: B-tree levels + order-index removes.
pub fn engine_with_btree_book() -> (EngineWithBook<BTreeOrderBook>, CollectingEventSink) {
    engine_with_book::<BTreeOrderBook>()
}

/// [`PoolLevelOrderBook`]: signed-price level pool.
pub fn engine_with_pool_level_book(
) -> (EngineWithBook<PoolLevelOrderBook>, CollectingEventSink) {
    engine_with_book::<PoolLevelOrderBook>()
}

/// [`DashSkipOrderBook`]: DashMap levels + SkipMap best price (Phase 1 path).
pub fn engine_with_dash_skip_book(
) -> (EngineWithBook<DashSkipOrderBook>, CollectingEventSink) {
    engine_with_book::<DashSkipOrderBook>()
}

#[allow(dead_code)]
pub fn engine_with_shared_state() -> (
    EngineWithSharedState,
    CollectingEventSink,
    SharedPriceBook,
    SharedOrderStore,
) {
    let seq = IncrementalSequence::default();
    let book_inner = Rc::new(RefCell::new(InMemoryPriceBook::default()));
    let book = SharedPriceBookHandle(book_inner.clone());
    let store_inner = Rc::new(RefCell::new(InMemoryOrderStore::default()));
    let store = SharedOrderStoreHandle(store_inner.clone());
    let matching = CrossingMatchingPolicy;
    let self_trade = AllowSelfTradePolicy;
    let sink = CollectingEventSink::default();
    let sink_clone = sink.clone();
    let engine =
        OrderMatchingEngine::new(seq, book, store, matching, self_trade, sink);
    (engine, sink_clone, book_inner, store_inner)
}

#[derive(Clone, Debug)]
pub struct EventRecord {
    pub event: EventSnapshot,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum EventSnapshot {
    Accepted(OrderId),
    Rejected(String),
    Canceled(OrderId),
    Trade {
        aggressor: OrderId,
        resting: OrderId,
        price: Price,
        quantity: Quantity,
    },
}

impl CollectingEventSink {
    pub fn snapshot(&self) -> Vec<EventRecord> {
        self.events()
            .iter()
            .map(|e| EventRecord {
                event: match e {
                    Event::Accepted(id) => EventSnapshot::Accepted(*id),
                    Event::Rejected(r) => EventSnapshot::Rejected(r.to_string()),
                    Event::Canceled(id) => EventSnapshot::Canceled(*id),
                    Event::Trade(t) => EventSnapshot::Trade {
                        aggressor: t.aggressor,
                        resting: t.resting,
                        price: t.price,
                        quantity: t.quantity,
                    },
                },
            })
            .collect()
    }
}

pub fn add_cmd(
    id: OrderId,
    participant_id: u64,
    side: Side,
    order_type: crate::types::OrderType,
    price: Option<Price>,
    quantity: Quantity,
    time_in_force: crate::types::TimeInForce,
) -> crate::engine::AddOrderCommand {
    crate::engine::AddOrderCommand {
        id,
        participant_id,
        symbol_id: 1,
        side,
        order_type,
        price,
        quantity,
        time_in_force,
        stop_price: None,
        max_visible_quantity: None,
        slippage: None,
        trailing_distance: None,
        trailing_step: None,
    }
}
