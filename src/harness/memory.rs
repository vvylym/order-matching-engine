#![allow(missing_docs)]

//! In-memory book and store for harness tests and benches.

use crate::book::PriceBook;
use crate::store::{OrderStore, OrderStoreError};
use crate::types::{Order, OrderId, Price, Quantity, Side};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;

type PriceLevel = BTreeMap<Price, VecDeque<Order>>;

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

#[allow(dead_code)]
pub type SharedPriceBook = Rc<RefCell<InMemoryPriceBook>>;

#[allow(dead_code)]
pub struct SharedPriceBookHandle(pub SharedPriceBook);

impl PriceBook for SharedPriceBookHandle {
    fn best_bid(&self) -> Option<Price> {
        PriceBook::best_bid(&*self.0.borrow())
    }
    fn best_ask(&self) -> Option<Price> {
        PriceBook::best_ask(&*self.0.borrow())
    }
    fn push(&mut self, price: &Price, order: &Order) {
        PriceBook::push(&mut *self.0.borrow_mut(), price, order)
    }
    fn pop_best(&mut self, side: Side) -> Option<Order> {
        PriceBook::pop_best(&mut *self.0.borrow_mut(), side)
    }
    fn remove(&mut self, order_id: &OrderId) -> Option<Order> {
        PriceBook::remove(&mut *self.0.borrow_mut(), order_id)
    }
}

#[allow(dead_code)]
pub type SharedOrderStore = Rc<RefCell<InMemoryOrderStore>>;

#[allow(dead_code)]
pub struct SharedOrderStoreHandle(pub SharedOrderStore);

impl OrderStore for SharedOrderStoreHandle {
    fn insert(&mut self, order: &Order) -> Result<(), OrderStoreError> {
        OrderStore::insert(&mut *self.0.borrow_mut(), order)
    }
    fn remove(&mut self, order_id: &OrderId) -> Result<Order, OrderStoreError> {
        OrderStore::remove(&mut *self.0.borrow_mut(), order_id)
    }
    fn get(&self, order_id: &OrderId) -> Option<Order> {
        OrderStore::get(&*self.0.borrow(), order_id)
    }
}
