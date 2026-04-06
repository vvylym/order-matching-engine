//! Events module
//!
#[cfg(test)]
use mockall::automock;

use crate::{
    error::RejectionError,
    types::{OrderId, Trade},
};

/// Event sink that discards all events (e.g. for ingestion tests).
#[derive(Debug, Clone, Default)]
pub struct NoOpEventSink;

impl EventSink for NoOpEventSink {
    fn emit(&self, _event: Event) -> Result<(), EventSinkError> {
        Ok(())
    }
}

/// Event sink trait
#[cfg_attr(test, automock)]
pub trait EventSink {
    /// Emit an event
    fn emit(&self, event: Event) -> Result<(), EventSinkError>;
}

/// Event
#[derive(Debug, Clone)]
pub enum Event {
    /// Order accepted
    Accepted(OrderId),
    /// Order rejected
    Rejected(RejectionError),
    /// Order canceled
    Canceled(OrderId),
    /// Trade
    Trade(Trade),
}

/// Event sink error
#[derive(Debug, thiserror::Error)]
pub enum EventSinkError {
    /// Disconnected
    #[error("disconnected")]
    Disconnected,
    /// Backpressure
    #[error("backpressure")]
    Backpressure,
    /// Serialization failed
    #[error("serialization failed: {0}")]
    SerializationFailed(String),
    /// Unexpected error
    #[error("unexpected error: {0}")]
    UnexpectedError(String),
}
