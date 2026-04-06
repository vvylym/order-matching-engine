//! Matching module
//!
#[cfg(test)]
use mockall::automock;

use crate::types::{Order, Price, Side};

/// Matching policy that allows a match when sides are opposite and price crosses.
/// Use for tests and integration (e.g. ITCH ingestion).
#[derive(Debug, Clone, Default)]
pub struct PriceCrossMatchingPolicy;

impl MatchingPolicy for PriceCrossMatchingPolicy {
    fn can_match(
        &self,
        incoming: &Order,
        resting: &Order,
    ) -> Result<bool, MatchingPolicyError> {
        if incoming.side == resting.side {
            return Err(MatchingPolicyError::IncompatibleSides);
        }
        let resting_price = match resting.price {
            Some(p) => p,
            None => {
                return Err(MatchingPolicyError::PriceDoesNotCross {
                    incoming_price: incoming.price,
                    resting_price: 0,
                });
            }
        };
        let incoming_price = incoming.price;
        let crosses = match (incoming.side, incoming_price) {
            (Side::Buy, Some(p)) => p >= resting_price,
            (Side::Sell, Some(p)) => p <= resting_price,
            (_, None) => return Err(MatchingPolicyError::UndefinedMarketPrice),
        };
        Ok(crosses)
    }
}

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
