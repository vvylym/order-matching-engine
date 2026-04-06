//! Engine module
//!

mod models;
mod service;

pub use models::*;
pub use service::*;

use crate::error::Result;

/// Order matching service trait (engine port).
/// Protocols (e.g. ITCH) produce [`OrderCommand`] and call [`Self::process`].
pub trait OrderMatchingService {
    /// Process an order matching command.
    fn process(&mut self, cmd: OrderCommand) -> Result<()> {
        match cmd {
            OrderCommand::Add(c) => self.add(c),
            OrderCommand::Cancel(c) => self.cancel(c),
            OrderCommand::Replace(c) => self.replace(c),
            OrderCommand::CancelByOrderId(c) => self.cancel_by_order_id(c),
            OrderCommand::Reduce(c) => self.reduce(c),
            OrderCommand::Execute(c) => self.execute(c),
            OrderCommand::ReplaceByNewId(c) => self.replace_by_new_id(c),
        }
    }

    /// Add an order.
    fn add(&mut self, cmd: AddOrderCommand) -> Result<()>;

    /// Cancel an order (participant and sequence must match).
    fn cancel(&mut self, cmd: CancelOrderCommand) -> Result<()>;

    /// Replace an order in place (same id, new price/quantity).
    fn replace(&mut self, cmd: ReplaceOrderCommand) -> Result<()>;

    /// Cancel by order id only (e.g. protocol feed; no ownership check).
    fn cancel_by_order_id(&mut self, cmd: CancelByOrderIdCommand) -> Result<()>;

    /// Reduce (partial cancel) an order by quantity.
    fn reduce(&mut self, cmd: ReduceOrderCommand) -> Result<()>;

    /// Execute (fill) a quantity of an order.
    fn execute(&mut self, cmd: ExecuteOrderCommand) -> Result<()>;

    /// Replace an order with a new id (remove old, add new).
    fn replace_by_new_id(
        &mut self,
        cmd: ReplaceOrderByNewIdCommand,
    ) -> Result<()>;
}
