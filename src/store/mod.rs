//! Store module
//!
#[cfg(test)]
use mockall::automock;

use crate::types::*;

/// Order store trait
#[cfg_attr(test, automock)]
pub trait OrderStore {
    /// Insert an order into the store
    fn insert(&mut self, order: &Order) -> Result<(), OrderStoreError>;
    /// Remove an order from the store
    fn remove(&mut self, order_id: &OrderId) -> Result<Order, OrderStoreError>;
    /// Get an order from the store
    fn get(&self, order_id: &OrderId) -> Option<Order>;
}

/// Order store error
#[derive(Debug, thiserror::Error)]
pub enum OrderStoreError {
    /// Order already exists
    #[error("order already exists: {0}")]
    AlreadyExists(OrderId),
    /// Order not found
    #[error("order not found: {0}")]
    NotFound(OrderId),
    /// Participant mismatch
    #[error(
        "participant mismatch: {order_id}, expected: {expected}, vs actual: {actual}"
    )]
    ParticipantMismatch {
        /// Order ID
        order_id: OrderId,
        /// Expected participant ID
        expected: ParticipantId,
        /// Actual participant ID
        actual: ParticipantId,
    },
    /// Corrupted state
    #[error("corrupted state: {0}")]
    CorruptedState(String), // invariant violation (should never happen)
    /// Unexpected error
    #[error("unexpected error: {0}")]
    UnexpectedError(String),
}
