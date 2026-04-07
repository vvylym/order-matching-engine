//! Order matching in Rust (**crate name:** `omer`).
//!
//! # What this crate does
//!
//! - Accepts commands such as **add / cancel / replace** (and variants like cancel-by-id or reduce)
//!   through the [`engine::OrderMatchingService`] trait.
//! - Keeps resting orders in a **[`book::PriceBook`]** and authoritative state in an **[`store::OrderStore`]**.
//! - Emits **[`events::Event`]**s (accepted, rejected, trade, canceled) through an **[`events::EventSink`]**.
//! - Optionally ingests **NASDAQ ITCH-style** bytes via **[`itch`]** ([`itch::process_itch_stream`],
//!   [`itch::process_itch_bytes`], [`itch::scan_decode_book_messages`] for decode-only profiling).
//!
//! # Where to start reading
//!
//! 1. [`types`] — `Order`, `Price`, `Quantity`, sides, time-in-force.
//! 2. [`engine`] — commands such as [`OrderCommand`](engine::OrderCommand) (see `engine/models.rs` in source).
//! 3. [`engine::OrderMatchingEngine`] — wires policies, book, and store; batch feeds via **[`OrderMatchingEngine::process_batch`](engine::OrderMatchingEngine::process_batch)**.
//!    For composition-based setup, use [`engine::builder()`](engine::builder).
//! 4. [`pool`] — optional `Order` shell recycling for adapters.
//! 5. **[`parallel`]** (feature **`parallel`**) — read-mostly `rayon` helpers; the book stays single-writer.
//!
//! # Tests and in-memory setup
//!
//! Integration tests build a real engine with in-memory structures. Those live under the
//! **`harness`** feature (enabled by default): **[`harness`]**, including [`harness::engine_with_book`]
//! to swap [`book::PriceBook`] implementations for comparable benches.
//!
//! Benchmark-harness binaries live under `src/bin/`:
//! - `server`: shard-aware Tokio harness endpoint
//! - `client`: load generator for local throughput/latency experiments
//!
//! # Unsafe
//!
//! The crate sets **`unsafe_code = forbid`**. Any exception must be minimal and documented with
//! `// SAFETY:` plus review.

pub mod book;
pub mod engine;
pub mod error;
pub mod events;
pub mod itch;
pub mod matching;
pub mod pool;
pub mod self_trade;
pub mod sequence;
pub mod store;
pub mod types;

#[cfg(feature = "parallel")]
pub mod parallel;

#[cfg(feature = "harness")]
pub mod harness;

#[cfg(test)]
mod tests;
