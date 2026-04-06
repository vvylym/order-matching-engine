//! Self trade module
//!
#[cfg(test)]
use mockall::automock;

use crate::types::{Order, OrderId, ParticipantId};

/// Self trade policy that never reports a violation (allow all).
/// Use for tests and integration (e.g. ITCH ingestion).
#[derive(Debug, Clone, Default)]
pub struct AllowAllSelfTradePolicy;

impl SelfTradePolicy for AllowAllSelfTradePolicy {
    fn violates(
        &self,
        _incoming: &Order,
        _resting: &Order,
    ) -> Result<bool, SelfTradePolicyError> {
        Ok(false)
    }
}

/// Self trade prevention trait
#[cfg_attr(test, automock)]
pub trait SelfTradePolicy {
    /// Check if an order is a self trade
    fn violates(
        &self,
        incoming: &Order,
        resting: &Order,
    ) -> Result<bool, SelfTradePolicyError>;
}

/// Self trade prevention policy error
#[derive(Debug, thiserror::Error)]
pub enum SelfTradePolicyError {
    /// Same participant
    #[error(
        "same participant: {participant_id}, aggressor: {aggressor}, resting: {resting}"
    )]
    SameParticipant {
        /// Participant ID
        participant_id: ParticipantId,
        /// Aggressor order ID
        aggressor: OrderId,
        /// Resting order ID
        resting: OrderId,
    },
    /// Policy misconfigured
    #[error("policy misconfigured: {0}")]
    PolicyMisconfigured(String),
    /// Unexpected error
    #[error("unexpected error: {0}")]
    UnexpectedError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Order, OrderId, OrderType, ParticipantId, Side, TimeInForce,
    };

    fn dummy_order(id: OrderId, participant_id: ParticipantId) -> Order {
        Order {
            symbol_id: 1,
            id,
            participant_id,
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(100),
            quantity: 1,
            time_in_force: TimeInForce::Gtc,
            executed_quantity: 0,
            leaves_quantity: 1,
            sequence: 0,
            ..Order::default()
        }
    }

    #[test]
    fn allow_all_never_violates() {
        let pol = AllowAllSelfTradePolicy;
        let a = dummy_order(1, 99);
        let b = dummy_order(2, 99);
        assert!(!pol.violates(&a, &b).unwrap());
    }
}
