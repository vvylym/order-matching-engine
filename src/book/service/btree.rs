//! BTree-based price book (BTreeMap per side + Vec per level, FIFO).

use std::collections::{BTreeMap, HashMap};

use crate::book::PriceBook;
use crate::types::{Order, OrderId, Price, Side};

type PriceLevel = BTreeMap<Price, Vec<Order>>;
type OrderIndex = HashMap<OrderId, (Price, Side)>;

/// Price book with bids and asks as `BTreeMap<Price, Vec<Order>>`. Best bid = max key,
/// best ask = min key. FIFO within each level. An index speeds up remove by order id.
#[derive(Debug, Default)]
pub struct BTreeOrderBook {
    bids: PriceLevel,
    asks: PriceLevel,
    /// order_id -> (price, side) for O(1) remove
    index: OrderIndex,
}

impl BTreeOrderBook {
    /// Creates an empty book.
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            index: HashMap::new(),
        }
    }

    fn side_map_mut(&mut self, side: Side) -> &mut PriceLevel {
        match side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        }
    }
}

impl PriceBook for BTreeOrderBook {
    fn best_bid(&self) -> Option<Price> {
        self.bids.keys().next_back().copied()
    }

    fn best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }

    fn push(&mut self, price: &Price, order: &Order) {
        let side = order.side;
        self.index.insert(order.id, (*price, side));
        let level = self.side_map_mut(side).entry(*price).or_default();
        level.push(order.clone());
    }

    fn pop_best(&mut self, aggressor_side: Side) -> Option<Order> {
        let (_best_price, level_key) = match aggressor_side {
            Side::Buy => self.asks.keys().next().copied(),
            Side::Sell => self.bids.keys().next_back().copied(),
        }
        .map(|p| (p, p))?;
        let map = match aggressor_side {
            Side::Buy => &mut self.asks,
            Side::Sell => &mut self.bids,
        };
        let level = map.get_mut(&level_key)?;
        let order = level.remove(0);
        if level.is_empty() {
            map.remove(&level_key);
        }
        self.index.remove(&order.id);
        Some(order)
    }

    fn remove(&mut self, order_id: &OrderId) -> Option<Order> {
        let (price, side) = self.index.remove(order_id)?;
        let map = self.side_map_mut(side);
        let level = map.get_mut(&price)?;
        let pos = level.iter().position(|o| o.id == *order_id)?;
        let order = level.remove(pos);
        if level.is_empty() {
            map.remove(&price);
        }
        Some(order)
    }
}
