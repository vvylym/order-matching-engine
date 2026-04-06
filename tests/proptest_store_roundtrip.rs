//! Property: basic store insert/remove roundtrip holds for dense ids.

use omer::store::service::HashMapOrderStore;
use omer::store::{OrderStore, OrderStoreError};
use omer::types::{Order, OrderType, Side, TimeInForce};
use proptest::prelude::*;

fn order_with_id(id: u64) -> Order {
    Order {
        symbol_id: 0,
        id,
        participant_id: 1,
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(100),
        quantity: 10,
        time_in_force: TimeInForce::Gtc,
        leaves_quantity: 10,
        executed_quantity: 0,
        sequence: 0,
        ..Order::default()
    }
}

proptest! {
    #[test]
    fn hash_map_store_insert_remove_roundtrip(id in 1u64..10_000u64) {
        let mut s = HashMapOrderStore::new();
        let o = order_with_id(id);
        s.insert(&o).unwrap();
        assert_eq!(s.get(&id).unwrap().id, id);
        assert_eq!(s.remove(&id).unwrap().id, id);
        assert!(s.get(&id).is_none());
    }

    #[test]
    fn hash_map_store_double_insert_err(id in 1u64..10_000u64) {
        let mut s = HashMapOrderStore::new();
        let o = order_with_id(id);
        s.insert(&o).unwrap();
        let e = s.insert(&o).unwrap_err();
        prop_assert!(matches!(e, OrderStoreError::AlreadyExists(x) if x == id));
    }
}
