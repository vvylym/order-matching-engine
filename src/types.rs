//! Types module
//!

/// Unique identifier for an order
pub type OrderId = u64;

/// Unique identifier for a participant
pub type ParticipantId = u64;

/// Unique identifier for a symbol (stock / instrument)
pub type SymbolId = u32;

/// Price of an order (tick-aligned)
pub type Price = i64;

/// Quantity of an order (lot-aligned)
pub type Quantity = i64;

/// Sequence number for ordering purposes
pub type Sequence = u64;

/// Symbol description (minimal for now; extend as needed)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Symbol {
    /// Internal identifier
    pub id: SymbolId,
    /// Short name (e.g. 8-char NASDAQ symbol)
    pub name: [u8; 8],
}

/// Side of an order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Buy side
    Buy,
    /// Sell side
    Sell,
}

/// Type of an order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    /// Limit order
    Limit,
    /// Market order
    Market,
    /// Stop order (triggers a market order when stop price is reached)
    Stop,
    /// Stop-limit order (triggers a limit order when stop price is reached)
    StopLimit,
    /// Trailing stop order (stop price trails the market)
    TrailingStop,
    /// Trailing stop-limit order (limit price trails the market)
    TrailingStopLimit,
}

/// Time-in-Force of an order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInForce {
    /// Good Till Cancel (GTC)
    Gtc,
    /// Immediate Or Cancel (IOC)
    Ioc,
    /// Fill Or Kill (FOK)
    Fok,
    /// All Or None (AON)
    Aon,
}

/// An order
#[derive(Debug, Clone)]
pub struct Order {
    /// Symbol identifier
    pub symbol_id: SymbolId,
    /// Unique identifier for the order
    pub id: OrderId,
    /// Unique identifier for the participant
    pub participant_id: ParticipantId,
    /// Side of the order
    pub side: Side,
    /// Type of the order
    pub order_type: OrderType,
    /// Price of the order (None for market orders)
    pub price: Option<Price>,
    /// Quantity of the order
    pub quantity: Quantity,
    /// Time-in-Force of the order
    pub time_in_force: TimeInForce,
    /// Optional stop price (for stop / stop-limit / trailing orders)
    pub stop_price: Option<Price>,
    /// Maximum visible quantity (for iceberg / hidden orders). None = fully visible.
    pub max_visible_quantity: Option<Quantity>,
    /// Slippage tolerance (price ticks the aggressor is willing to cross)
    pub slippage: Option<Price>,
    /// Trailing offset from reference price (for trailing orders)
    pub trailing_distance: Option<Price>,
    /// Trailing step (minimum move before adjusting trailing price)
    pub trailing_step: Option<Price>,
    /// Executed quantity so far
    pub executed_quantity: Quantity,
    /// Remaining quantity (leaves). Should satisfy quantity = executed_quantity + leaves_quantity.
    pub leaves_quantity: Quantity,
    /// Sequence number of the order
    pub sequence: Sequence,
}

/// Trade (Clone for event replay and tests)
#[derive(Debug, Clone)]
pub struct Trade {
    /// Aggressor order
    pub aggressor: OrderId,
    /// Resting order
    pub resting: OrderId,
    /// Price of the trade
    pub price: Price,
    /// Quantity of the trade
    pub quantity: Quantity,
    /// Sequence number of the trade
    pub sequence: Sequence,
}
