//! Order matching commands
//!

use crate::types::*;

/// Order matching commands (engine port).
/// Protocols (e.g. ITCH) decode wire messages into these variants; the engine is the single consumer.
#[derive(Debug, Clone)]
pub enum OrderCommand {
    /// Add an order
    Add(AddOrderCommand),
    /// Cancel an order (requires participant and sequence for ownership).
    Cancel(CancelOrderCommand),
    /// Replace an order in place (same id, new price/quantity).
    Replace(ReplaceOrderCommand),
    /// Cancel by order id only (e.g. protocol feed delete; no participant/sequence check).
    CancelByOrderId(CancelByOrderIdCommand),
    /// Reduce (partial cancel) an order by quantity.
    Reduce(ReduceOrderCommand),
    /// Execute (fill) a quantity of an order.
    Execute(ExecuteOrderCommand),
    /// Replace an order with a new id (e.g. ITCH replace: old_id → new_id, new price/quantity).
    ReplaceByNewId(ReplaceOrderByNewIdCommand),
}

/// Cancel by order id only (for protocol feeds that do not send participant/sequence).
#[derive(Debug, Clone)]
pub struct CancelByOrderIdCommand {
    /// Order id to cancel.
    pub order_id: OrderId,
}

/// Reduce (partial cancel) by quantity.
#[derive(Debug, Clone)]
pub struct ReduceOrderCommand {
    /// Order id to reduce.
    pub order_id: OrderId,
    /// Quantity to cancel.
    pub quantity: Quantity,
}

/// Execute (fill) a quantity of an order.
#[derive(Debug, Clone)]
pub struct ExecuteOrderCommand {
    /// Order id executed.
    pub order_id: OrderId,
    /// Executed quantity.
    pub quantity: Quantity,
}

/// Replace order with a new id (protocol semantics: old order removed, new order added).
#[derive(Debug, Clone)]
pub struct ReplaceOrderByNewIdCommand {
    /// Existing order id to replace.
    pub old_order_id: OrderId,
    /// New order id.
    pub new_order_id: OrderId,
    /// New limit price.
    pub new_price: Price,
    /// New quantity.
    pub new_quantity: Quantity,
    /// Symbol (from original order when `None`, e.g. ITCH replace).
    pub symbol_id: Option<SymbolId>,
    /// Side (from original order when `None`, e.g. ITCH replace).
    pub side: Option<Side>,
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
