//! Order Matching Engine
//!
pub mod book;
pub mod engine;
pub mod error;
pub mod events;
pub mod itch;
pub mod matching;
pub mod self_trade;
pub mod sequence;
pub mod store;
pub mod types;

#[cfg(feature = "harness")]
pub mod harness;

#[cfg(test)]
mod tests;
