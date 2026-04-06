//! Concrete price book implementations (ports).
//!
//! Use `omer::book::service::BTreeOrderBook` or `omer::book::service::PoolLevelOrderBook`;
//! no need to export these from the parent module.

mod btree;
mod pool_level;

pub use btree::BTreeOrderBook;
pub use pool_level::PoolLevelOrderBook;

#[cfg(test)]
mod tests {
    use crate::book::PriceBook;
    use crate::types::{Order, OrderId, OrderType, Price, Side, TimeInForce};

    fn order(id: OrderId, side: Side, price: Price, qty: i64) -> Order {
        Order {
            symbol_id: 0,
            id,
            participant_id: 0,
            side,
            order_type: OrderType::Limit,
            price: Some(price),
            quantity: qty,
            time_in_force: TimeInForce::Gtc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
            executed_quantity: 0,
            leaves_quantity: qty,
            sequence: id,
        }
    }

    /// Single test entry point: runs the same PriceBook contract tests for any implementation.
    fn run_price_book_tests<B: PriceBook + Default>() {
        let book = B::default();
        assert!(book.best_bid().is_none());
        assert!(book.best_ask().is_none());

        let mut book = B::default();
        let o1 = order(1, Side::Buy, 100, 10);
        let o2 = order(2, Side::Buy, 101, 5);
        book.push(&100, &o1);
        book.push(&101, &o2);
        assert_eq!(book.best_bid(), Some(101));
        assert!(book.best_ask().is_none());
        book.push(&200, &order(3, Side::Sell, 200, 1));
        assert_eq!(book.best_ask(), Some(200));

        let mut book = B::default();
        book.push(&100, &order(1, Side::Buy, 100, 10));
        book.push(&100, &order(2, Side::Buy, 100, 5));
        assert_eq!(book.pop_best(Side::Buy).unwrap().id, 1);
        assert_eq!(book.pop_best(Side::Buy).unwrap().id, 2);
        assert!(book.pop_best(Side::Buy).is_none());

        let mut book = B::default();
        book.push(&100, &order(1, Side::Buy, 100, 10));
        book.push(&100, &order(2, Side::Buy, 100, 5));
        let r = book.remove(&1).unwrap();
        assert_eq!(r.id, 1);
        assert_eq!(book.pop_best(Side::Buy).unwrap().id, 2);
    }

    #[test]
    fn btree_order_book() {
        run_price_book_tests::<super::BTreeOrderBook>();
    }

    #[test]
    fn pool_level_order_book() {
        run_price_book_tests::<super::PoolLevelOrderBook>();
    }
}
