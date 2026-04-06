//! Pool-based price book with signed prices (bid positive, ask negative).

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::book::PriceBook;
use crate::types::{OrderId, Price, Sequence, Side};

type OrderIndex = HashMap<OrderId, (i64, usize)>;

/// Signed price: positive = bid, negative = ask. One comparison for side+price.
#[inline]
fn signed_price(price: Price, side: Side) -> i64 {
    match side {
        Side::Buy => price,
        Side::Sell => -price,
    }
}

/// One price level: FIFO list of order ids (payload in [`crate::store::OrderStore`]).
#[derive(Debug, Clone, Default)]
struct Level {
    signed_price: i64,
    orders: Vec<OrderId>,
}

/// Pool of levels (alloc/free, no per-level heap beyond the vec).
#[derive(Debug, Default)]
struct Pool {
    allocated: Vec<Level>,
    free: Vec<usize>,
}

impl Pool {
    fn new() -> Self {
        Self {
            allocated: Vec::new(),
            free: Vec::new(),
        }
    }

    fn alloc(&mut self, signed_price: i64) -> usize {
        if let Some(i) = self.free.pop() {
            self.allocated[i].signed_price = signed_price;
            self.allocated[i].orders.clear();
            i
        } else {
            let i = self.allocated.len();
            self.allocated.push(Level {
                signed_price,
                orders: Vec::new(),
            });
            i
        }
    }

    fn free(&mut self, idx: usize) {
        self.free.push(idx);
    }

    fn get_mut(&mut self, idx: usize) -> Option<&mut Level> {
        self.allocated.get_mut(idx)
    }
}

/// Price book using a level pool and signed prices. Bids stored as positive,
/// asks as negative; best bid = max positive key, best ask = min absolute of negative keys.
#[derive(Debug)]
pub struct PoolLevelOrderBook {
    /// Signed price -> level index in pool. Bid keys > 0, ask keys < 0.
    levels: BTreeMap<i64, usize>,
    pool: Pool,
    /// order_id -> (signed_price, level_idx) for O(1) remove
    index: OrderIndex,
}

impl Default for PoolLevelOrderBook {
    fn default() -> Self {
        Self::new()
    }
}

impl PoolLevelOrderBook {
    /// Creates an empty book.
    pub fn new() -> Self {
        Self {
            levels: BTreeMap::new(),
            pool: Pool::new(),
            index: HashMap::new(),
        }
    }
}

impl PriceBook for PoolLevelOrderBook {
    fn best_bid(&self) -> Option<Price> {
        self.levels.range(0..).next_back().map(|(&k, _)| k)
    }

    fn best_ask(&self) -> Option<Price> {
        self.levels.range(..0).next_back().map(|(&k, _)| -k)
    }

    fn push(
        &mut self,
        price: &Price,
        order_id: OrderId,
        side: Side,
        _time_priority: Sequence,
    ) {
        let sp = signed_price(*price, side);
        let idx = if let Some(&idx) = self.levels.get(&sp) {
            self.pool.get_mut(idx).unwrap().orders.push(order_id);
            idx
        } else {
            let idx = self.pool.alloc(sp);
            self.pool.get_mut(idx).unwrap().orders.push(order_id);
            self.levels.insert(sp, idx);
            idx
        };
        self.index.insert(order_id, (sp, idx));
    }

    fn pop_best(&mut self, aggressor_side: Side) -> Option<OrderId> {
        let (best_key, level_idx) = match aggressor_side {
            Side::Buy => self.levels.range(..0).next_back(),
            Side::Sell => self.levels.range(0..).next_back(),
        }
        .map(|(k, &v)| (*k, v))?;
        let level = self.pool.get_mut(level_idx)?;
        let id = level.orders.remove(0);
        if level.orders.is_empty() {
            self.levels.remove(&best_key);
            self.pool.free(level_idx);
        }
        self.index.remove(&id);
        Some(id)
    }

    fn remove(&mut self, order_id: &OrderId) -> bool {
        let Some((sp, level_idx)) = self.index.remove(order_id) else {
            return false;
        };
        let level = match self.pool.get_mut(level_idx) {
            Some(l) => l,
            None => return false,
        };
        let pos = match level.orders.iter().position(|id| id == order_id) {
            Some(p) => p,
            None => return false,
        };
        level.orders.remove(pos);
        if level.orders.is_empty() {
            self.levels.remove(&sp);
            self.pool.free(level_idx);
        }
        true
    }
}
