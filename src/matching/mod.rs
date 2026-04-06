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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Order, OrderId, OrderType, Side, TimeInForce};

    fn limit(side: Side, price: i64, id: OrderId) -> Order {
        Order {
            symbol_id: 1,
            id,
            participant_id: 1,
            side,
            order_type: OrderType::Limit,
            price: Some(price),
            quantity: 10,
            time_in_force: TimeInForce::Gtc,
            executed_quantity: 0,
            leaves_quantity: 10,
            sequence: 0,
            ..Order::default()
        }
    }

    #[test]
    fn price_cross_same_side_is_incompatible() {
        let p = PriceCrossMatchingPolicy;
        let buy50 = limit(Side::Buy, 50, 1);
        let buy51 = limit(Side::Buy, 51, 2);
        assert!(matches!(
            p.can_match(&buy51, &buy50),
            Err(MatchingPolicyError::IncompatibleSides)
        ));
    }

    #[test]
    fn price_cross_buy_aggressor_crosses_higher_resting_sell() {
        let p = PriceCrossMatchingPolicy;
        let sell = limit(Side::Sell, 50, 1);
        let buy = limit(Side::Buy, 50, 2);
        assert!(p.can_match(&buy, &sell).unwrap());
        let buy_below = limit(Side::Buy, 49, 3);
        assert!(!p.can_match(&buy_below, &sell).unwrap());
    }

    #[test]
    fn price_cross_sell_aggressor_crosses_lower_resting_buy() {
        let p = PriceCrossMatchingPolicy;
        let buy = limit(Side::Buy, 50, 1);
        let sell = limit(Side::Sell, 50, 2);
        assert!(p.can_match(&sell, &buy).unwrap());
        let sell_above = limit(Side::Sell, 51, 3);
        assert!(!p.can_match(&sell_above, &buy).unwrap());
    }

    #[test]
    fn market_incoming_without_price_is_undefined() {
        let p = PriceCrossMatchingPolicy;
        let mut m = limit(Side::Buy, 50, 1);
        m.price = None;
        let sell = limit(Side::Sell, 50, 2);
        assert!(matches!(
            p.can_match(&m, &sell),
            Err(MatchingPolicyError::UndefinedMarketPrice)
        ));
    }

    #[test]
    fn resting_market_order_price_reports_does_not_cross() {
        let p = PriceCrossMatchingPolicy;
        let buy = limit(Side::Buy, 50, 1);
        let mut sell_no_px = limit(Side::Sell, 50, 2);
        sell_no_px.price = None;
        assert!(matches!(
            p.can_match(&buy, &sell_no_px),
            Err(MatchingPolicyError::PriceDoesNotCross { .. })
        ));
    }
}
