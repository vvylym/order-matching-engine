//! Concrete order store implementations (ports).
//!
//! Use `omer::store::service::HashMapOrderStore` or `omer::store::service::DenseOrderStore`;
//! no need to export these from the parent module.

mod dense;
mod hash_map;

pub use dense::DenseOrderStore;
pub use hash_map::HashMapOrderStore;

#[cfg(test)]
mod tests {
    use crate::store::{OrderStore, OrderStoreError};
    use crate::types::{
        Order, OrderId, OrderType, ParticipantId, Side, TimeInForce,
    };

    fn sample_order(id: OrderId, participant_id: ParticipantId) -> Order {
        Order {
            symbol_id: 0,
            id,
            participant_id,
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(100),
            quantity: 10,
            time_in_force: TimeInForce::Gtc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
            executed_quantity: 0,
            leaves_quantity: 10,
            sequence: 1,
        }
    }

    /// Single test entry point: runs the same OrderStore contract tests for any implementation.
    fn run_order_store_tests<S: OrderStore + Default>() {
        let mut store = S::default();
        let order = sample_order(1, 100);
        store.insert(&order).unwrap();
        assert!(store.get(&1).is_some());
        assert_eq!(store.get(&1).unwrap().quantity, 10);
        let removed = store.remove(&1).unwrap();
        assert_eq!(removed.id, 1);
        assert!(store.get(&1).is_none());

        let mut store2 = S::default();
        let order2 = sample_order(1, 100);
        store2.insert(&order2).unwrap();
        let e = store2.insert(&order2).unwrap_err();
        assert!(matches!(e, OrderStoreError::AlreadyExists(1)));

        let mut store3 = S::default();
        let e = store3.remove(&999).unwrap_err();
        assert!(matches!(e, OrderStoreError::NotFound(999)));
    }

    #[test]
    fn hash_map_order_store() {
        run_order_store_tests::<super::HashMapOrderStore>();
    }

    #[test]
    fn dense_order_store() {
        run_order_store_tests::<super::DenseOrderStore>();
    }

    #[test]
    fn dense_order_store_reserve() {
        let mut store = super::DenseOrderStore::new();
        store.reserve(1000);
        let order = sample_order(1000, 0);
        store.insert(&order).unwrap();
        assert_eq!(store.get(&1000).unwrap().id, 1000);
    }
}
