//! **Phase-1 book:** per-side [`SkipMap`] for ordered **price levels** (best bid / best ask)
//! and [`DashMap`] for **level queues** (`Price` → `Vec<OrderId>`), plus a HashMap index for cancel-by-id.
//!
//! This layout matches the roadmap: concurrent-friendly maps for later parallel batches, while
//! [`crate::book::PriceBook`] stays single-writer `&mut self` today.

use std::collections::HashMap;
use std::fmt;

use crossbeam_skiplist::SkipMap;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry as DEntry;

use crate::book::PriceBook;
use crate::types::{OrderId, Price, Sequence, Side};

type LevelQueue = Vec<OrderId>;
type OrderLocation = (Price, Side);

/// Single side: skip list of active prices + dash map of FIFO queues at each price.
#[derive(Default)]
struct BookSide {
    /// Ascending prices that currently have ≥1 order (`()` is a placeholder value).
    prices: SkipMap<Price, ()>,
    levels: DashMap<Price, LevelQueue>,
}

impl BookSide {
    fn push(&mut self, price: Price, order_id: OrderId) {
        match self.levels.entry(price) {
            DEntry::Occupied(mut o) => {
                o.get_mut().push(order_id);
            }
            DEntry::Vacant(v) => {
                v.insert(vec![order_id]);
                self.prices.insert(price, ());
            }
        }
    }

    fn pop_best_buy(&mut self) -> Option<OrderId> {
        let price = self.prices.back().map(|e| *e.key())?;
        let mut level = self.levels.get_mut(&price)?;
        let order_id = level.remove(0);
        if level.is_empty() {
            drop(level);
            self.levels.remove(&price);
            self.prices.remove(&price);
        }
        Some(order_id)
    }

    fn pop_best_sell(&mut self) -> Option<OrderId> {
        let price = self.prices.front().map(|e| *e.key())?;
        let mut level = self.levels.get_mut(&price)?;
        let order_id = level.remove(0);
        if level.is_empty() {
            drop(level);
            self.levels.remove(&price);
            self.prices.remove(&price);
        }
        Some(order_id)
    }

    fn remove_order(&mut self, price: Price, order_id: &OrderId) -> bool {
        let mut level = match self.levels.get_mut(&price) {
            Some(l) => l,
            None => return false,
        };
        let pos = match level.iter().position(|id| id == order_id) {
            Some(p) => p,
            None => return false,
        };
        level.remove(pos);
        if level.is_empty() {
            drop(level);
            self.levels.remove(&price);
            self.prices.remove(&price);
        }
        true
    }

    fn best_buy(&self) -> Option<Price> {
        self.prices.back().map(|e| *e.key())
    }

    fn best_sell(&self) -> Option<Price> {
        self.prices.front().map(|e| *e.key())
    }
}

/// Price book: **SkipMap** for best-price queries, **DashMap** for per-level order queues.
pub struct DashSkipOrderBook {
    bids: BookSide,
    asks: BookSide,
    index: HashMap<OrderId, OrderLocation>,
}

impl Default for DashSkipOrderBook {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DashSkipOrderBook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DashSkipOrderBook")
            .field("bids_best", &self.bids.best_buy())
            .field("asks_best", &self.asks.best_sell())
            .field("indexed_orders", &self.index.len())
            .finish()
    }
}

impl DashSkipOrderBook {
    /// Empty book.
    pub fn new() -> Self {
        Self {
            bids: BookSide::default(),
            asks: BookSide::default(),
            index: HashMap::new(),
        }
    }
}

impl PriceBook for DashSkipOrderBook {
    fn best_bid(&self) -> Option<Price> {
        self.bids.best_buy()
    }

    fn best_ask(&self) -> Option<Price> {
        self.asks.best_sell()
    }

    fn push(
        &mut self,
        price: &Price,
        order_id: OrderId,
        side: Side,
        _time_priority: Sequence,
    ) {
        self.index.insert(order_id, (*price, side));
        match side {
            Side::Buy => self.bids.push(*price, order_id),
            Side::Sell => self.asks.push(*price, order_id),
        }
    }

    fn pop_best(&mut self, aggressor_side: Side) -> Option<OrderId> {
        let order_id = match aggressor_side {
            Side::Buy => self.asks.pop_best_sell(),
            Side::Sell => self.bids.pop_best_buy(),
        }?;
        self.index.remove(&order_id);
        Some(order_id)
    }

    fn remove(&mut self, order_id: &OrderId) -> bool {
        let Some((price, side)) = self.index.remove(order_id) else {
            return false;
        };
        match side {
            Side::Buy => self.bids.remove_order(price, order_id),
            Side::Sell => self.asks.remove_order(price, order_id),
        }
    }
}
