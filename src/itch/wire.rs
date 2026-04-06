//! ITCH 5.0 wire types and big-endian read helpers.
#![allow(missing_docs)]

use crate::error::FeedError;

/// Nanosecond timestamp (6 bytes on wire, big-endian).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Timestamp(pub u64);

/// Order id on wire (8 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Oid(pub u64);

/// Price on wire (4 bytes, unsigned).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WirePrice(pub u32);

/// Quantity on wire (4 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WireQty(pub u32);

/// Stock locate / book identifier (2 bytes on wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StockLocate(pub u16);

/// Buy or Sell on wire (ASCII 'B' / 'S').
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BuySell {
    Buy = b'B',
    Sell = b'S',
}

impl TryFrom<u8> for BuySell {
    type Error = ();
    fn try_from(b: u8) -> Result<Self, ()> {
        match b {
            b'B' => Ok(BuySell::Buy),
            b'S' => Ok(BuySell::Sell),
            _ => Err(()),
        }
    }
}

fn parse_error(required: usize, got: usize) -> FeedError {
    FeedError::Parse { required, got }
}

/// Read big-endian u16 at offset.
pub fn read_u16_be(bytes: &[u8], offset: usize) -> Result<u16, FeedError> {
    let end = offset + 2;
    if bytes.len() < end {
        return Err(parse_error(end, bytes.len()));
    }
    Ok(u16::from_be_bytes([bytes[offset], bytes[offset + 1]]))
}

/// Read big-endian u32 at offset.
pub fn read_u32_be(bytes: &[u8], offset: usize) -> Result<u32, FeedError> {
    let end = offset + 4;
    if bytes.len() < end {
        return Err(parse_error(end, bytes.len()));
    }
    Ok(u32::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]))
}

/// Read big-endian u64 at offset.
pub fn read_u64_be(bytes: &[u8], offset: usize) -> Result<u64, FeedError> {
    let end = offset + 8;
    if bytes.len() < end {
        return Err(parse_error(end, bytes.len()));
    }
    Ok(u64::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ]))
}

/// Read 6-byte big-endian timestamp at offset.
pub fn read_timestamp(
    bytes: &[u8],
    offset: usize,
) -> Result<Timestamp, FeedError> {
    let end = offset + 6;
    if bytes.len() < end {
        return Err(parse_error(end, bytes.len()));
    }
    let high = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]) as u64;
    let low = u32::from_be_bytes([
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
    ]) as u64;
    Ok(Timestamp((high << 32) | low))
}

/// Read 8-byte order id at offset.
pub fn read_oid(bytes: &[u8], offset: usize) -> Result<Oid, FeedError> {
    read_u64_be(bytes, offset).map(Oid)
}

/// Read 4-byte price at offset.
pub fn read_price(bytes: &[u8], offset: usize) -> Result<WirePrice, FeedError> {
    read_u32_be(bytes, offset).map(WirePrice)
}

/// Read 4-byte quantity at offset.
pub fn read_qty(bytes: &[u8], offset: usize) -> Result<WireQty, FeedError> {
    read_u32_be(bytes, offset).map(WireQty)
}

/// Read 2-byte stock locate at offset.
pub fn read_locate(
    bytes: &[u8],
    offset: usize,
) -> Result<StockLocate, FeedError> {
    read_u16_be(bytes, offset).map(StockLocate)
}
