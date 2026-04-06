//! In-memory implementations of PriceBook and OrderStore for integration tests.

use omer::book::PriceBook;
use omer::store::OrderStore;
use omer::store::OrderStoreError;
use omer::types::{Order, OrderId, Price, Quantity, Side};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;

/// One price level: FIFO queue of orders (reduces type complexity for clippy).
type PriceLevel = BTreeMap<Price, VecDeque<Order>>;

/// In-memory price book: price-time priority (FIFO per level).
#[derive(Default)]
pub struct InMemoryPriceBook {
    bids: PriceLevel,
    asks: PriceLevel,
}

impl InMemoryPriceBook {
    fn push_side(
        side: Side,
        price: Price,
        order: &Order,
        bids: &mut PriceLevel,
        asks: &mut PriceLevel,
    ) {
        match side {
            Side::Buy => bids.entry(price).or_default().push_back(order.clone()),
            Side::Sell => asks.entry(price).or_default().push_back(order.clone()),
        }
    }

    /// Pop the order with the smallest sequence at the best price (price-time priority).
    fn pop_best_side(
        side: Side,
        bids: &mut PriceLevel,
        asks: &mut PriceLevel,
    ) -> Option<Order> {
        let (best_price, q) = match side {
            Side::Buy => {
                let best_ask = asks.keys().next().copied()?;
                let q = asks.get_mut(&best_ask)?;
                (best_ask, q)
            }
            Side::Sell => {
                let best_bid = bids.keys().next_back().copied()?;
                let q = bids.get_mut(&best_bid)?;
                (best_bid, q)
            }
        };
        let idx = q
            .iter()
            .enumerate()
            .min_by_key(|(_, o)| o.sequence)
            .map(|(i, _)| i)?;
        let order = q.remove(idx)?;
        if q.is_empty() {
            match side {
                Side::Buy => {
                    asks.remove(&best_price);
                }
                Side::Sell => {
                    bids.remove(&best_price);
                }
            }
        }
        Some(order)
    }

    fn remove_from_side(
        order_id: &OrderId,
        bids: &mut PriceLevel,
        asks: &mut PriceLevel,
    ) -> Option<Order> {
        for q in bids.values_mut() {
            if let Some(pos) = q.iter().position(|o| o.id == *order_id) {
                return Some(q.remove(pos).unwrap());
            }
        }
        for q in asks.values_mut() {
            if let Some(pos) = q.iter().position(|o| o.id == *order_id) {
                return Some(q.remove(pos).unwrap());
            }
        }
        None
    }
}

impl PriceBook for InMemoryPriceBook {
    fn best_bid(&self) -> Option<Price> {
        self.bids.keys().next_back().copied()
    }

    fn best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }

    fn push(&mut self, price: &Price, order: &Order) {
        Self::push_side(
            order.side,
            *price,
            order,
            &mut self.bids,
            &mut self.asks,
        );
    }

    fn pop_best(&mut self, side: Side) -> Option<Order> {
        Self::pop_best_side(side, &mut self.bids, &mut self.asks)
    }

    fn remove(&mut self, order_id: &OrderId) -> Option<Order> {
        Self::remove_from_side(order_id, &mut self.bids, &mut self.asks)
    }
}

/// In-memory order store.
#[derive(Default)]
pub struct InMemoryOrderStore {
    orders: HashMap<OrderId, Order>,
}

impl OrderStore for InMemoryOrderStore {
    fn insert(&mut self, order: &Order) -> Result<(), OrderStoreError> {
        if self.orders.contains_key(&order.id) {
            return Err(OrderStoreError::AlreadyExists(order.id));
        }
        self.orders.insert(order.id, order.clone());
        Ok(())
    }

    fn remove(&mut self, order_id: &OrderId) -> Result<Order, OrderStoreError> {
        self.orders
            .remove(order_id)
            .ok_or(OrderStoreError::NotFound(*order_id))
    }

    fn get(&self, order_id: &OrderId) -> Option<Order> {
        self.orders.get(order_id).cloned()
    }
}

impl InMemoryPriceBook {
    /// Total quantity resting in the book (for VIII.2).
    #[allow(dead_code)]
    pub fn total_depth(&self) -> Quantity {
        let bid_qty: Quantity = self
            .bids
            .values()
            .flat_map(|q| q.iter())
            .map(|o| o.quantity)
            .sum();
        let ask_qty: Quantity = self
            .asks
            .values()
            .flat_map(|q| q.iter())
            .map(|o| o.quantity)
            .sum();
        bid_qty + ask_qty
    }
}

/// Shared handle to an in-memory book so tests can inspect state after engine runs.
#[allow(dead_code)]
pub type SharedPriceBook = Rc<RefCell<InMemoryPriceBook>>;

/// Wrapper that implements PriceBook and delegates to shared inner book.
#[allow(dead_code)]
pub struct SharedPriceBookHandle(pub SharedPriceBook);

impl PriceBook for SharedPriceBookHandle {
    fn best_bid(&self) -> Option<Price> {
        omer::book::PriceBook::best_bid(&*self.0.borrow())
    }
    fn best_ask(&self) -> Option<Price> {
        omer::book::PriceBook::best_ask(&*self.0.borrow())
    }
    fn push(&mut self, price: &Price, order: &Order) {
        omer::book::PriceBook::push(&mut *self.0.borrow_mut(), price, order)
    }
    fn pop_best(&mut self, side: Side) -> Option<Order> {
        omer::book::PriceBook::pop_best(&mut *self.0.borrow_mut(), side)
    }
    fn remove(&mut self, order_id: &OrderId) -> Option<Order> {
        omer::book::PriceBook::remove(&mut *self.0.borrow_mut(), order_id)
    }
}

/// Shared handle to an in-memory store for test inspection.
#[allow(dead_code)]
pub type SharedOrderStore = Rc<RefCell<InMemoryOrderStore>>;

/// Wrapper that implements OrderStore and delegates to shared inner store.
#[allow(dead_code)]
pub struct SharedOrderStoreHandle(pub SharedOrderStore);

impl OrderStore for SharedOrderStoreHandle {
    fn insert(&mut self, order: &Order) -> Result<(), OrderStoreError> {
        omer::store::OrderStore::insert(&mut *self.0.borrow_mut(), order)
    }
    fn remove(&mut self, order_id: &OrderId) -> Result<Order, OrderStoreError> {
        omer::store::OrderStore::remove(&mut *self.0.borrow_mut(), order_id)
    }
    fn get(&self, order_id: &OrderId) -> Option<Order> {
        omer::store::OrderStore::get(&*self.0.borrow(), order_id)
    }
}
