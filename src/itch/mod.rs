//! ITCH 5.0 protocol decoder.
//!
//! Parses ITCH wire format into [`crate::engine::OrderCommand`]s and applies them via
//! [`crate::engine::OrderMatchingService`]. Use [`process_itch_stream`] for `Read` input,
//! [`process_itch_bytes`] for an in-memory slice, and [`scan_decode_book_messages`] when profiling
//! decode only (no engine).

mod buf;
mod messages;
mod stream;
mod wire;

pub use stream::{
    decode_book_message, process_itch_bytes, process_itch_stream,
    scan_decode_book_messages,
};
pub use wire::{BuySell, Oid, StockLocate, Timestamp, WirePrice, WireQty};

// Re-export for tests or advanced use
pub use buf::BufferedReader;
pub use messages::{AddOrder, ItchMsgType, message_len};
