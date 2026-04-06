//! Book module
//!

pub mod service;

#[cfg(test)]
use mockall::automock;

use crate::types::{Order, OrderId, Price, Side};

/// Price book trait
#[cfg_attr(test, automock)]
pub trait PriceBook {
    /// Get the best bid price
    fn best_bid(&self) -> Option<Price>;
    /// Get the best ask price
    fn best_ask(&self) -> Option<Price>;
    /// Push an order to the price book
    fn push(&mut self, price: &Price, order: &Order);
    /// Pop the best resting order **on the opposite side** to `side` (the **aggressor** side):
    /// incoming **buy** consumes **asks** (lowest ask first, FIFO at level); incoming **sell**
    /// consumes **bids** (highest bid first, FIFO at level).
    fn pop_best(&mut self, side: Side) -> Option<Order>;
    /// Remove an order from the price book
    fn remove(&mut self, order_id: &OrderId) -> Option<Order>;
}
