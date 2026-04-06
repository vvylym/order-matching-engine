//! Object pool for [`crate::types::Order`] shells (optional integration in hot clients).
//!
//! Resting order **payloads** live in [`crate::store::OrderStore`]; this pool only recycles
//! allocated `Order` structs when you build commands or temporary copies outside the engine.

use crate::types::Order;

/// Reuses `Order` allocations between logical lifetimes (fill/cancel/custom adapters).
#[derive(Debug, Default)]
pub struct OrderPool {
    free: Vec<Order>,
}

impl OrderPool {
    /// Returns a cleared shell from the pool or [`Order::default`].
    pub fn take(&mut self) -> Order {
        self.free.pop().unwrap_or_default()
    }

    /// Returns a used shell to the pool (replaced with a default instance for reuse).
    pub fn recycle(&mut self, _finished: Order) {
        self.free.push(Order::default());
    }

    /// Clears pooled entries (e.g. after a bounded test batch).
    pub fn clear(&mut self) {
        self.free.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OrderType, Side, TimeInForce};

    #[test]
    fn take_recycle_roundtrip() {
        let mut p = OrderPool::default();
        let mut o = p.take();
        o.id = 42;
        o.side = Side::Sell;
        o.order_type = OrderType::Limit;
        o.price = Some(100);
        o.quantity = 1;
        o.leaves_quantity = 1;
        o.time_in_force = TimeInForce::Gtc;
        p.recycle(o);
        let o2 = p.take();
        assert_eq!(o2.id, 0);
        assert_eq!(o2.side, Side::Buy);
    }
}
