//! ITCH 5.0 protocol decoder.
//!
//! This module parses ITCH wire format and produces [`crate::engine::OrderCommand`]s,
//! which are applied via [`crate::engine::OrderMatchingService`]. Other protocols
//! (e.g. OUCH) can be added the same way, producing `OrderCommand` and using the engine.

mod buf;
mod messages;
mod stream;
mod wire;

pub use stream::{decode_book_message, process_itch_stream};
pub use wire::{BuySell, Oid, StockLocate, Timestamp, WirePrice, WireQty};

// Re-export for tests or advanced use
pub use buf::BufferedReader;
pub use messages::{AddOrder, ItchMsgType, message_len};
