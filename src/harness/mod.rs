#![allow(missing_docs)]
#![allow(clippy::type_complexity)]

//! Shared in-memory **store**, **matching / self-trade policies**, and **event sink** for tests and benches.
//! Swap only the **`PriceBook`** via [`engine_with_book`] or the convenience `engine_with_*` helpers
//! so latency numbers stay comparable. For sustained throughput without event allocations, use
//! [`engine_with_book_noop`] and [`EngineWithBookNoOp`].
//!
//! Enabled via the **`harness`** crate feature (on by default). Use `default-features = false` to omit.

mod builders;
mod memory;
mod policy;
mod sink;

pub use builders::{
    EngineWithBook, EngineWithBookNoOp, EngineWithMemory,
    EngineWithSelfTradeRejection, EngineWithSharedState, EventRecord,
    EventSnapshot, IncrementalSequence, add_cmd, engine_with_book,
    engine_with_book_noop, engine_with_btree_book, engine_with_dash_skip_book,
    engine_with_memory, engine_with_pool_level_book,
    engine_with_self_trade_rejection, engine_with_shared_state,
};
pub use memory::{
    InMemoryOrderStore, InMemoryPriceBook, SharedOrderStore,
    SharedOrderStoreHandle, SharedPriceBook, SharedPriceBookHandle,
};
pub use policy::{
    AllowSelfTradePolicy, CrossingMatchingPolicy, RejectSelfTradePolicy,
};
pub use sink::CollectingEventSink;
