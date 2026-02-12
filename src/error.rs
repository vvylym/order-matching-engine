//! Error module
//!

use crate::events::EventSinkError;
use crate::matching::MatchingPolicyError;
use crate::self_trade::SelfTradePolicyError;
use crate::sequence::SequenceGeneratorError;
use crate::store::OrderStoreError;
use crate::types::OrderId;

/// Result type
pub type Result<T> = std::result::Result<T, Error>;

/// Error enum
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Sequence generator error
    #[error("sequence: {0}")]
    Sequence(#[from] SequenceGeneratorError),
    /// Order store error
    #[error("order store: {0}")]
    OrderStore(#[from] OrderStoreError),
    /// Matching policy error
    #[error("matching policy: {0}")]
    MatchingPolicy(#[from] MatchingPolicyError),
    /// Self trade policy error
    #[error("self trade policy: {0}")]
    SelfTradePolicy(#[from] SelfTradePolicyError),
    /// Event sink error
    #[error("event sink: {0}")]
    EventSink(#[from] EventSinkError),
    /// Rejection error
    #[error("rejection: {0}")]
    Rejection(#[from] RejectionError),
}

/// Rejection Error
#[derive(thiserror::Error, Debug, Clone, Copy)]
pub enum RejectionError {
    /// Invalid price
    #[error("invalid price")]
    InvalidPrice,
    /// Invalid quantity
    #[error("invalid quantity")]
    InvalidQuantity,
    /// Self trade
    #[error("self trade")]
    SelfTrade,
    /// Order not found
    #[error("order not found: {0}")]
    OrderNotFound(OrderId),
    /// Stale sequence
    #[error("stale sequence")]
    StaleSequence,
    /// Participant mismatch
    #[error("participant mismatch")]
    ParticipantMismatch,
    /// Invariant violation
    #[error("invariant violation")]
    PriceBookInvariantViolation,
    /// Insufficient liquidity
    #[error("insufficient liquidity")]
    InsufficientLiquidity,
    /// Symbol already exists
    #[error("symbol already exists")]
    SymbolDuplicate,
    /// Symbol not found
    #[error("symbol not found")]
    SymbolNotFound,
    /// Order book already exists
    #[error("order book already exists")]
    OrderBookDuplicate,
    /// Order book not found
    #[error("order book not found")]
    OrderBookNotFound,
    /// Order already exists
    #[error("order already exists")]
    OrderDuplicate,
    /// Invalid order id
    #[error("invalid order id")]
    OrderIdInvalid,
    /// Invalid order type
    #[error("invalid order type")]
    OrderTypeInvalid,
    /// Invalid order parameter (e.g. stop/limit combination)
    #[error("invalid order parameter")]
    OrderParameterInvalid,
    /// Invalid order quantity
    #[error("invalid order quantity")]
    OrderQuantityInvalid,
}
