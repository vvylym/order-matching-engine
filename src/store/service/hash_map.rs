//! HashMap-based order store (simple, good for tests and baseline).

use std::collections::HashMap;

use crate::store::{OrderStore, OrderStoreError};
use crate::types::{Order, OrderId};

/// Order store backed by a `HashMap<OrderId, Order>`. O(1) insert/remove/get.
#[derive(Debug, Default)]
pub struct HashMapOrderStore {
    map: HashMap<OrderId, Order>,
}

impl HashMapOrderStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl OrderStore for HashMapOrderStore {
    fn insert(&mut self, order: &Order) -> Result<(), OrderStoreError> {
        if self.map.contains_key(&order.id) {
            return Err(OrderStoreError::AlreadyExists(order.id));
        }
        self.map.insert(order.id, order.clone());
        Ok(())
    }

    fn remove(&mut self, order_id: &OrderId) -> Result<Order, OrderStoreError> {
        self.map
            .remove(order_id)
            .ok_or(OrderStoreError::NotFound(*order_id))
    }

    fn get(&self, order_id: &OrderId) -> Option<Order> {
        self.map.get(order_id).cloned()
    }
}
