//! Shared test fixtures, in-memory implementations, and engine builders.
//!
//! Use mocks for isolated engine behavior; use in-memory implementations
//! for end-to-end and invariant tests.

#![allow(dead_code)]
#![allow(clippy::type_complexity)]

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

use omer::engine::OrderMatchingEngine;
use omer::events::Event;
use omer::sequence::SequenceGenerator;
use omer::types::{OrderId, Price, Quantity, Sequence, Side};

/// Engine with in-memory components and allow self-trade (used by most tests).
pub type EngineWithMemory = OrderMatchingEngine<
    IncrementalSequence,
    InMemoryPriceBook,
    InMemoryOrderStore,
    CrossingMatchingPolicy,
    AllowSelfTradePolicy,
    CollectingEventSink,
>;

/// Engine with self-trade rejection (used by self_trade tests).
pub type EngineWithSelfTradeRejection = OrderMatchingEngine<
    IncrementalSequence,
    InMemoryPriceBook,
    InMemoryOrderStore,
    CrossingMatchingPolicy,
    RejectSelfTradePolicy,
    CollectingEventSink,
>;

/// Engine with shared book/store for inspection (used by book_integrity tests).
pub type EngineWithSharedState = OrderMatchingEngine<
    IncrementalSequence,
    SharedPriceBookHandle,
    SharedOrderStoreHandle,
    CrossingMatchingPolicy,
    AllowSelfTradePolicy,
    CollectingEventSink,
>;

/// Deterministic sequence generator for tests.
/// For replace, uses high sequence values so replaced orders go to the back of the queue.
#[derive(Default)]
pub struct IncrementalSequence {
    /// Next sequence for normal (add) orders: 0, 1, 2, ...
    pub next: Sequence,
    /// Counter for next_at_end_of_queue: replaced orders get MAX - 1 - back_count.
    back_count: Sequence,
}

impl IncrementalSequence {
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
    ) -> Result<Sequence, omer::sequence::SequenceGeneratorError> {
        let s = self.next;
        self.next = self.next.saturating_add(1);
        Ok(s)
    }

    fn next_at_end_of_queue(
        &mut self,
    ) -> Result<Sequence, omer::sequence::SequenceGeneratorError> {
        let c = self.back_count;
        self.back_count = self.back_count.saturating_add(1);
        Ok(Sequence::MAX.saturating_sub(1).saturating_sub(c))
    }
}

/// Build an engine with in-memory book/store and RejectSelfTradePolicy (for VII tests).
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

/// Build an engine with in-memory book/store and collecting event sink.
/// Use for integration tests that need to assert on final state and events.
pub fn engine_with_memory() -> (EngineWithMemory, CollectingEventSink) {
    let seq = IncrementalSequence::default();
    let book = InMemoryPriceBook::default();
    let store = InMemoryOrderStore::default();
    let matching = CrossingMatchingPolicy;
    let self_trade = AllowSelfTradePolicy;
    let sink = CollectingEventSink::default();
    let sink_clone = sink.clone();
    let engine =
        OrderMatchingEngine::new(seq, book, store, matching, self_trade, sink);
    (engine, sink_clone)
}

/// Engine plus shared book/store for invariant tests (VIII).
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

/// Copy of events for assertions (clone-friendly).
#[derive(Clone, Debug)]
pub struct EventRecord {
    pub event: EventSnapshot,
}

/// Snapshot of event for test assertions (fields used in pattern matching by tests).
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
    /// Return a snapshot of events for assertion (no dependency on omer::Event).
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

/// Helper to build default add command.
pub fn add_cmd(
    id: OrderId,
    participant_id: u64,
    side: Side,
    order_type: omer::types::OrderType,
    price: Option<Price>,
    quantity: Quantity,
    time_in_force: omer::types::TimeInForce,
) -> omer::engine::AddOrderCommand {
    omer::engine::AddOrderCommand {
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
