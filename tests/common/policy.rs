//! Matching and self-trade policies for tests.

use omer::matching::{MatchingPolicy, MatchingPolicyError};
use omer::self_trade::{SelfTradePolicy, SelfTradePolicyError};
use omer::types::Order;

/// Matching policy: can match when prices cross (buy price >= ask, sell price <= bid).
pub struct CrossingMatchingPolicy;

impl MatchingPolicy for CrossingMatchingPolicy {
    fn can_match(
        &self,
        incoming: &Order,
        resting: &Order,
    ) -> Result<bool, MatchingPolicyError> {
        let (in_price, rest_price) = match (incoming.price, resting.price) {
            (Some(ip), Some(rp)) => (ip, rp),
            (None, Some(_rp)) => {
                // Market order: always crosses resting limit
                return Ok(true);
            }
            _ => return Err(MatchingPolicyError::UndefinedMarketPrice),
        };
        let crosses = match (incoming.side, resting.side) {
            (omer::types::Side::Buy, omer::types::Side::Sell) => {
                in_price >= rest_price
            }
            (omer::types::Side::Sell, omer::types::Side::Buy) => {
                in_price <= rest_price
            }
            _ => return Err(MatchingPolicyError::IncompatibleSides),
        };
        Ok(crosses)
    }
}

/// Self-trade policy that never blocks (for tests that don't care about self-trade).
#[allow(dead_code)]
pub struct AllowSelfTradePolicy;

impl SelfTradePolicy for AllowSelfTradePolicy {
    fn violates(
        &self,
        _incoming: &Order,
        _resting: &Order,
    ) -> Result<bool, SelfTradePolicyError> {
        Ok(false)
    }
}

/// Self-trade policy that rejects when same participant (for VII tests).
#[allow(dead_code)]
pub struct RejectSelfTradePolicy;

impl SelfTradePolicy for RejectSelfTradePolicy {
    fn violates(
        &self,
        incoming: &Order,
        resting: &Order,
    ) -> Result<bool, SelfTradePolicyError> {
        Ok(incoming.participant_id == resting.participant_id)
    }
}
