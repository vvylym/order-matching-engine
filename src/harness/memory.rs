#![allow(missing_docs)]

//! In-memory book and store for harness tests and benches.

use crate::book::PriceBook;
use crate::store::{OrderStore, OrderStoreError};
use crate::types::{Order, OrderId, Price, Quantity, Sequence, Side};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

type PriceLevel = BTreeMap<Price, LevelQueue>;

#[derive(Default)]
struct LevelQueue {
    /// Queue entries ordered by time priority at [`Self::push`], `(sequence, order_id)`.
    ///
    /// Sequence is stored first so `pop_min_sequence` scans compare the hot key before the id.
    entries: Vec<(Sequence, OrderId)>,
    /// Slot indices are `u32` to shrink the hot `HashMap` value footprint on 64-bit targets.
    ///
    /// Harness invariant: at most `u32::MAX` resting orders per price level (matches practical limits).
    positions: HashMap<OrderId, u32>,
}

impl LevelQueue {
    fn push(&mut self, order_id: OrderId, time_priority: Sequence) {
        let idx_u32 = u32::try_from(self.entries.len()).expect(
            "in-memory level queue exceeded u32::MAX entries (harness instrument limit)",
        );
        self.entries.push((time_priority, order_id));
        self.positions.insert(order_id, idx_u32);
    }

    fn pop_min_sequence(&mut self) -> Option<OrderId> {
        let idx = self
            .entries
            .iter()
            .enumerate()
            .min_by_key(|(_, (seq, _))| *seq)
            .map(|(i, _)| i)?;
        self.remove_at(idx).map(|(_, order_id)| order_id)
    }

    fn remove_by_id(
        &mut self,
        order_id: &OrderId,
    ) -> Option<(OrderId, Sequence)> {
        let idx = *self.positions.get(order_id)?;
        let idx_usize = usize::try_from(idx).ok()?;
        self.remove_at(idx_usize).map(|(seq, id)| (id, seq))
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn iter_order_ids(&self) -> impl Iterator<Item = OrderId> + '_ {
        self.entries.iter().map(|(_, order_id)| *order_id)
    }

    fn remove_at(&mut self, idx: usize) -> Option<(Sequence, OrderId)> {
        let last_idx = self.entries.len().checked_sub(1)?;
        self.entries.swap(idx, last_idx);
        let removed = self.entries.pop()?;
        self.positions.remove(&removed.1);

        if idx < self.entries.len() {
            let moved_order_id = self.entries[idx].1;
            let idx_u32 = u32::try_from(idx)
                .expect("slot index must fit u32 (harness level size invariant)");
            self.positions.insert(moved_order_id, idx_u32);
        }
        Some(removed)
    }
}

#[derive(Default)]
pub struct InMemoryPriceBook {
    bids: PriceLevel,
    asks: PriceLevel,
    order_index: HashMap<OrderId, (Side, Price)>,
}

impl InMemoryPriceBook {
    fn push_side(
        side: Side,
        price: Price,
        order_id: OrderId,
        time_priority: Sequence,
        bids: &mut PriceLevel,
        asks: &mut PriceLevel,
        order_index: &mut HashMap<OrderId, (Side, Price)>,
    ) {
        match side {
            Side::Buy => {
                bids.entry(price).or_default().push(order_id, time_priority)
            }
            Side::Sell => {
                asks.entry(price).or_default().push(order_id, time_priority)
            }
        }
        order_index.insert(order_id, (side, price));
    }

    fn pop_best_side(
        side: Side,
        bids: &mut PriceLevel,
        asks: &mut PriceLevel,
        order_index: &mut HashMap<OrderId, (Side, Price)>,
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
        let order_id = q.pop_min_sequence()?;
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
        order_index.remove(&order_id);
        Some(order_id)
    }

    fn remove_from_level(
        order_id: &OrderId,
        side: Side,
        price: Price,
        bids: &mut PriceLevel,
        asks: &mut PriceLevel,
    ) -> bool {
        let levels = match side {
            Side::Buy => bids,
            Side::Sell => asks,
        };
        let should_remove_level = if let Some(q) = levels.get_mut(&price) {
            if q.remove_by_id(order_id).is_some() {
                q.is_empty()
            } else {
                return false;
            }
        } else {
            return false;
        };
        if should_remove_level {
            levels.remove(&price);
        }
        true
    }

    fn remove_from_side(
        order_id: &OrderId,
        bids: &mut PriceLevel,
        asks: &mut PriceLevel,
        order_index: &mut HashMap<OrderId, (Side, Price)>,
    ) -> bool {
        let Some((side, price)) = order_index.remove(order_id) else {
            return false;
        };
        if Self::remove_from_level(order_id, side, price, bids, asks) {
            return true;
        }

        // Keep behavior stable if index and queues drift apart unexpectedly.
        order_index.insert(*order_id, (side, price));
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
            &mut self.order_index,
        );
    }

    fn pop_best(&mut self, side: Side) -> Option<OrderId> {
        Self::pop_best_side(
            side,
            &mut self.bids,
            &mut self.asks,
            &mut self.order_index,
        )
    }

    fn remove(&mut self, order_id: &OrderId) -> bool {
        Self::remove_from_side(
            order_id,
            &mut self.bids,
            &mut self.asks,
            &mut self.order_index,
        )
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
        let mut bid_qty: Quantity = 0;
        for level in self.bids.values() {
            for order_id in level.iter_order_ids() {
                if let Some(qty) = resolve(order_id) {
                    bid_qty += qty;
                }
            }
        }
        let mut ask_qty: Quantity = 0;
        for level in self.asks.values() {
            for order_id in level.iter_order_ids() {
                if let Some(qty) = resolve(order_id) {
                    ask_qty += qty;
                }
            }
        }
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
