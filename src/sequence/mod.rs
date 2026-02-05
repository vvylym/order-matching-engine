//! Sequence module
//!
#[cfg(test)]
use mockall::automock;

use crate::types::Sequence;

/// Sequence generator trait
#[cfg_attr(test, automock)]
pub trait SequenceGenerator {
    /// Generate a new sequence (e.g. for new orders; FIFO at same price).
    fn next(&mut self) -> Result<Sequence, SequenceGeneratorError>;

    /// Generate a sequence that sorts at the end of the queue (e.g. for replaced orders that must lose time priority).
    /// Default implementation uses `next()`; implementors may override to assign "back of queue".
    fn next_at_end_of_queue(
        &mut self,
    ) -> Result<Sequence, SequenceGeneratorError> {
        self.next()
    }
}

/// Sequence generator error
#[derive(Debug, thiserror::Error)]
pub enum SequenceGeneratorError {
    /// Sequence overflow
    #[error("Sequence overflow")]
    Overflow,
    /// Corrupted state
    #[error("Corrupted state: {0}")]
    CorruptedState(String), // invariant violation (should never happen)
    /// Unexpected error
    #[error("unexpected error: {0}")]
    UnexpectedError(String),
}
