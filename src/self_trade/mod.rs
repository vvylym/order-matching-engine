//! Self trade module
//!
#[cfg(test)]
use mockall::automock;

use crate::types::{Order, OrderId, ParticipantId};

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
