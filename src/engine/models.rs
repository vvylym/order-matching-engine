//! Order matching commands
//!

use crate::types::*;

/// Order matching commands
#[derive(Debug, Clone)]
pub enum OrderCommand {
    /// Add an order
    Add(AddOrderCommand),
    /// Cancel an order
    Cancel(CancelOrderCommand),
    /// Replace an order
    Replace(ReplaceOrderCommand),
}

/// Add order command
#[derive(Debug, Clone)]
pub struct AddOrderCommand {
    /// Unique identifier for the order
    pub id: OrderId,
    /// Unique identifier for the participant
    pub participant_id: ParticipantId,
    /// Symbol identifier
    pub symbol_id: SymbolId,
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
}

/// Cancel order command
#[derive(Debug, Clone)]
pub struct CancelOrderCommand {
    /// Unique identifier for the order
    pub order_id: OrderId,
    /// Unique identifier for the participant
    pub participant_id: ParticipantId,
    /// Sequence number of the order
    pub sequence: Sequence,
}

/// Replace order command
#[derive(Debug, Clone)]
pub struct ReplaceOrderCommand {
    /// Unique identifier for the order
    pub order_id: OrderId,
    /// Unique identifier for the participant
    pub participant_id: ParticipantId,
    /// New price of the order (None for market orders)
    pub new_price: Option<Price>,
    /// New quantity of the order
    pub new_quantity: Quantity,
    /// Sequence number of the order
    pub sequence: Sequence,
}
