//! Dense order store indexed by OrderId (O(1) lookup; itch-style).

use crate::store::{OrderStore, OrderStoreError};
use crate::types::{Order, OrderId};

type OrderSlot = Vec<Option<Order>>;

/// Order store backed by a `Vec<Option<Order>>` indexed by OrderId. O(1) lookup when
/// capacity is pre-reserved; good for performance when order ids are dense.
#[derive(Debug)]
pub struct DenseOrderStore {
    data: OrderSlot,
}

impl Default for DenseOrderStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DenseOrderStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Ensures capacity for at least `order_id + 1` (so index `order_id` is valid).
    pub fn reserve(&mut self, order_id: OrderId) {
        let idx = order_id as usize;
        if idx >= self.data.len() {
            self.data.resize(idx + 1, None);
        }
    }
}

impl OrderStore for DenseOrderStore {
    fn insert(&mut self, order: &Order) -> Result<(), OrderStoreError> {
        self.reserve(order.id);
        let idx = order.id as usize;
        if self.data[idx].is_some() {
            return Err(OrderStoreError::AlreadyExists(order.id));
        }
        self.data[idx] = Some(order.clone());
        Ok(())
    }

    fn remove(&mut self, order_id: &OrderId) -> Result<Order, OrderStoreError> {
        let idx = *order_id as usize;
        if idx >= self.data.len() {
            return Err(OrderStoreError::NotFound(*order_id));
        }
        self.data[idx]
            .take()
            .ok_or(OrderStoreError::NotFound(*order_id))
    }

    fn get(&self, order_id: &OrderId) -> Option<Order> {
        let idx = *order_id as usize;
        self.data.get(idx).and_then(|o| o.as_ref()).cloned()
    }
}
