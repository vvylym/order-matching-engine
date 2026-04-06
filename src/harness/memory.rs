#![allow(missing_docs)]

//! In-memory book and store for harness tests and benches.

use crate::book::PriceBook;
use crate::store::{OrderStore, OrderStoreError};
use crate::types::{Order, OrderId, Price, Quantity, Sequence, Side};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;

type PriceLevel = BTreeMap<Price, VecDeque<(OrderId, Sequence)>>;

#[derive(Default)]
pub struct InMemoryPriceBook {
    bids: PriceLevel,
    asks: PriceLevel,
}

impl InMemoryPriceBook {
    fn push_side(
        side: Side,
        price: Price,
        order_id: OrderId,
        time_priority: Sequence,
        bids: &mut PriceLevel,
        asks: &mut PriceLevel,
    ) {
        match side {
            Side::Buy => bids
                .entry(price)
                .or_default()
                .push_back((order_id, time_priority)),
            Side::Sell => asks
                .entry(price)
                .or_default()
                .push_back((order_id, time_priority)),
        }
    }

    fn pop_best_side(
        side: Side,
        bids: &mut PriceLevel,
        asks: &mut PriceLevel,
    ) -> Option<OrderId> {
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
            .min_by_key(|(_, (_, seq))| *seq)
            .map(|(i, _)| i)?;
        let (order_id, _) = q.remove(idx)?;
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
        Some(order_id)
    }

    fn remove_from_side(
        order_id: &OrderId,
        bids: &mut PriceLevel,
        asks: &mut PriceLevel,
    ) -> bool {
        for q in bids.values_mut() {
            if let Some(pos) = q.iter().position(|(id, _)| id == order_id) {
                q.remove(pos).unwrap();
                return true;
            }
        }
        for q in asks.values_mut() {
            if let Some(pos) = q.iter().position(|(id, _)| id == order_id) {
                q.remove(pos).unwrap();
                return true;
            }
        }
        false
    }
}

impl PriceBook for InMemoryPriceBook {
    fn best_bid(&self) -> Option<Price> {
        self.bids.keys().next_back().copied()
    }

    fn best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }

    fn push(
        &mut self,
        price: &Price,
        order_id: OrderId,
        side: Side,
        time_priority: Sequence,
    ) {
        Self::push_side(
            side,
            *price,
            order_id,
            time_priority,
            &mut self.bids,
            &mut self.asks,
        );
    }

    fn pop_best(&mut self, side: Side) -> Option<OrderId> {
        Self::pop_best_side(side, &mut self.bids, &mut self.asks)
    }

    fn remove(&mut self, order_id: &OrderId) -> bool {
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
    /// Sum of `leaves_quantity` for ids queued in the book, resolved via `resolve` (usually [`OrderStore::get`]).
    pub fn total_depth<F>(&self, mut resolve: F) -> Quantity
    where
        F: FnMut(OrderId) -> Option<Quantity>,
    {
        let bid_qty: Quantity = self
            .bids
            .values()
            .flat_map(|q| q.iter())
            .filter_map(|&(id, _)| resolve(id))
            .sum();
        let ask_qty: Quantity = self
            .asks
            .values()
            .flat_map(|q| q.iter())
            .filter_map(|&(id, _)| resolve(id))
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
    fn push(
        &mut self,
        price: &Price,
        order_id: OrderId,
        side: Side,
        time_priority: Sequence,
    ) {
        PriceBook::push(
            &mut *self.0.borrow_mut(),
            price,
            order_id,
            side,
            time_priority,
        )
    }
    fn pop_best(&mut self, side: Side) -> Option<OrderId> {
        PriceBook::pop_best(&mut *self.0.borrow_mut(), side)
    }
    fn remove(&mut self, order_id: &OrderId) -> bool {
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
