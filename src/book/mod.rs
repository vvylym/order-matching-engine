//! Book module
//!

pub mod service;

#[cfg(test)]
use mockall::automock;

use crate::types::{OrderId, Price, Sequence, Side};

/// Price book trait
#[cfg_attr(test, automock)]
pub trait PriceBook {
    /// Get the best bid price
    fn best_bid(&self) -> Option<Price>;
    /// Get the best ask price
    fn best_ask(&self) -> Option<Price>;
    /// Queue `order_id` at `price` on `side`. Canonical [`crate::types::Order`] must already live in [`crate::store::OrderStore`].
    /// `time_priority` is the order's sequence (FIFO backends ignore it; in-memory test book uses it to order the level).
    fn push(
        &mut self,
        price: &Price,
        order_id: OrderId,
        side: Side,
        time_priority: Sequence,
    );
    /// Pop the best resting **order id** on the opposite side to `side` (the **aggressor** side):
    /// incoming **buy** consumes **asks** (lowest ask first, FIFO at level); incoming **sell**
    /// consumes **bids** (highest bid first, FIFO at level).
    fn pop_best(&mut self, side: Side) -> Option<OrderId>;
    /// Remove `order_id` from the book. Returns whether the id was present in a level queue.
    fn remove(&mut self, order_id: &OrderId) -> bool;
}
