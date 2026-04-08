use std::sync::Arc;

type BytesTx = crossbeam_channel::Sender<Vec<u8>>;

use crate::error::RejectionError;
use crate::events::{Event, EventSink, EventSinkError};

type EncodeResult = Result<Vec<u8>, String>;

/// Serialize-to-bytes + bounded-channel sink.
///
/// This is intended for harness and production-ish measurements:
/// - **serialize cost** is included (no-op is unrealistically cheap),
/// - **bounded channel** models backpressure,
/// - a consumer thread can drain the receiver to emulate an async publisher.
#[derive(Clone)]
pub struct BytesChannelEventSink {
    tx: BytesTx,
    codec: Arc<EventCodec>,
}

impl BytesChannelEventSink {
    /// Create a sink that tries to send serialized event bytes to `tx`.
    pub fn new(tx: BytesTx) -> Self {
        Self {
            tx,
            codec: Arc::new(EventCodec),
        }
    }
}

impl std::fmt::Debug for BytesChannelEventSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BytesChannelEventSink")
            .finish_non_exhaustive()
    }
}

impl EventSink for BytesChannelEventSink {
    fn emit(&self, event: Event) -> Result<(), EventSinkError> {
        let bytes = self
            .codec
            .encode(&event)
            .map_err(EventSinkError::SerializationFailed)?;
        self.tx.try_send(bytes).map_err(|e| match e {
            crossbeam_channel::TrySendError::Full(_) => {
                EventSinkError::Backpressure
            }
            crossbeam_channel::TrySendError::Disconnected(_) => {
                EventSinkError::Disconnected
            }
        })
    }
}

/// Simple stable binary encoding for events.
///
/// Format (little-endian):
/// - `Accepted`: tag=1, order_id(u64)
/// - `Canceled`: tag=2, order_id(u64)
/// - `Rejected`: tag=3, code(u8)
/// - `Trade`: tag=4, aggressor(u64), resting(u64), price(i64), qty(i64), seq(u64)
struct EventCodec;

impl EventCodec {
    fn encode(&self, event: &Event) -> EncodeResult {
        let mut out = Vec::with_capacity(1 + 8 * 6);
        match event {
            Event::Accepted(oid) => {
                out.push(1);
                put_u64(&mut out, *oid);
            }
            Event::Canceled(oid) => {
                out.push(2);
                put_u64(&mut out, *oid);
            }
            Event::Rejected(rej) => {
                out.push(3);
                out.push(rejection_code(*rej));
            }
            Event::Trade(t) => {
                out.push(4);
                put_u64(&mut out, t.aggressor);
                put_u64(&mut out, t.resting);
                put_i64(&mut out, t.price);
                put_i64(&mut out, t.quantity);
                put_u64(&mut out, t.sequence);
            }
        }
        Ok(out)
    }
}

fn rejection_code(r: RejectionError) -> u8 {
    // Stable mapping for measurement payloads (not a public protocol guarantee).
    match r {
        RejectionError::InvalidPrice => 1,
        RejectionError::InvalidQuantity => 2,
        RejectionError::SelfTrade => 3,
        RejectionError::OrderNotFound(_) => 4,
        RejectionError::StaleSequence => 5,
        RejectionError::ParticipantMismatch => 6,
        RejectionError::PriceBookInvariantViolation => 7,
        RejectionError::InsufficientLiquidity => 8,
        RejectionError::SymbolDuplicate => 9,
        RejectionError::SymbolNotFound => 10,
        RejectionError::OrderBookDuplicate => 11,
        RejectionError::OrderBookNotFound => 12,
        RejectionError::OrderDuplicate => 13,
        RejectionError::OrderIdInvalid => 14,
        RejectionError::OrderTypeInvalid => 15,
        RejectionError::OrderParameterInvalid => 16,
        RejectionError::OrderQuantityInvalid => 17,
    }
}

fn put_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn put_i64(buf: &mut Vec<u8>, v: i64) {
    buf.extend_from_slice(&v.to_le_bytes());
}
