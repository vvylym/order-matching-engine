//! Types module
//!

/// Unique identifier for an order
pub type OrderId = u64;

/// Unique identifier for a participant
pub type ParticipantId = u64;

/// Price of an order (tick-aligned)
pub type Price = i64;

/// Quantity of an order (lot-aligned)
pub type Quantity = i64;

/// Sequence number for ordering purposes
pub type Sequence = u64;

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
}

/// Time-in-Force of an order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInForce {
    /// Good Till Cancel (GTC)
    Gtc,
    /// Immediate Or Cancel (IOC)
    Ioc,
}

/// An order
#[derive(Debug, Clone)]
pub struct Order {
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
