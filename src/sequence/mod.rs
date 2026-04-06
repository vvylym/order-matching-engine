//! Sequence module
//!
#[cfg(test)]
use mockall::automock;

use crate::types::Sequence;

/// Simple sequence generator that increments from 0.
/// Use for tests and integration (e.g. ITCH ingestion).
#[derive(Debug, Default)]
pub struct CounterSequenceGenerator {
    next: Sequence,
}

impl CounterSequenceGenerator {
    /// Creates a new counter starting at 0.
    pub fn new() -> Self {
        Self { next: 0 }
    }
}

impl SequenceGenerator for CounterSequenceGenerator {
    fn next(&mut self) -> Result<Sequence, SequenceGeneratorError> {
        let v = self.next;
        self.next = self
            .next
            .checked_add(1)
            .ok_or(SequenceGeneratorError::Overflow)?;
        Ok(v)
    }
}

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
