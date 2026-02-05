//! Matching module
//!
#[cfg(test)]
use mockall::automock;

use crate::types::{Order, Price};

/// Matching policy trait
#[cfg_attr(test, automock)]
pub trait MatchingPolicy {
    /// Check if an order can match with a resting order
    fn can_match(
        &self,
        incoming: &Order,
        resting: &Order,
    ) -> Result<bool, MatchingPolicyError>;
}

/// Matching policy error
#[derive(Debug, thiserror::Error)]
pub enum MatchingPolicyError {
    /// Incompatible sides
    #[error("incompatible sides")]
    IncompatibleSides,
    /// Price does not cross
    #[error(
        "price does not cross: incoming: {incoming_price:?} vs resting: {resting_price}"
    )]
    PriceDoesNotCross {
        /// Incoming price
        incoming_price: Option<Price>,
        /// Resting price
        resting_price: Price,
    },
    /// Undefined market price (market vs empty book)
    #[error("undefined market price")]
    UndefinedMarketPrice,
    /// Invalid order type combination
    #[error("invalid order type combination")]
    InvalidOrderTypeCombination,
    /// Unexpected error
    #[error("unexpected error: {0}")]
    UnexpectedError(String),
}
