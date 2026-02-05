//! Engine module
//!

mod models;

pub use models::*;

use crate::error::Result;

/// Order matching service trait
pub trait OrderMatchingService {
    /// Process an order matching command
    fn process(&mut self, cmd: OrderCommand) -> Result<()> {
        match cmd {
            OrderCommand::Add(cmd) => self.add(cmd),
            OrderCommand::Cancel(cmd) => self.cancel(cmd),
            OrderCommand::Replace(cmd) => self.replace(cmd),
        }
    }

    /// Add an order
    fn add(&mut self, cmd: AddOrderCommand) -> Result<()>;

    /// Cancel an order
    fn cancel(&mut self, cmd: CancelOrderCommand) -> Result<()>;

    /// Replace an order
    fn replace(&mut self, cmd: ReplaceOrderCommand) -> Result<()>;
}
