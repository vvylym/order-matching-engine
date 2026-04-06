#![allow(missing_docs)]

//! Matching and self-trade policies for harness tests and benches.

use crate::matching::{MatchingPolicy, MatchingPolicyError};
use crate::self_trade::{SelfTradePolicy, SelfTradePolicyError};
use crate::types::Order;

/// Matching when prices cross (incl. market aggressor vs limit resting).
pub struct CrossingMatchingPolicy;

impl MatchingPolicy for CrossingMatchingPolicy {
    fn can_match(
        &self,
        incoming: &Order,
        resting: &Order,
    ) -> Result<bool, MatchingPolicyError> {
        let (in_price, rest_price) = match (incoming.price, resting.price) {
            (Some(ip), Some(rp)) => (ip, rp),
            (None, Some(_)) => return Ok(true),
            _ => return Err(MatchingPolicyError::UndefinedMarketPrice),
        };
        let crosses = match (incoming.side, resting.side) {
            (crate::types::Side::Buy, crate::types::Side::Sell) => {
                in_price >= rest_price
            }
            (crate::types::Side::Sell, crate::types::Side::Buy) => {
                in_price <= rest_price
            }
            _ => return Err(MatchingPolicyError::IncompatibleSides),
        };
        Ok(crosses)
    }
}

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
