//! ITCH 5.0 message types and payload parsing.
#![allow(missing_docs)]

pub mod wire {
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
    #[inline]
    pub fn read_u16_be(bytes: &[u8], offset: usize) -> Result<u16, FeedError> {
        let end = offset + 2;
        if bytes.len() < end {
            return Err(parse_error(end, bytes.len()));
        }
        Ok(u16::from_be_bytes([bytes[offset], bytes[offset + 1]]))
    }

    /// Read big-endian u32 at offset.
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
    pub fn read_oid(bytes: &[u8], offset: usize) -> Result<Oid, FeedError> {
        read_u64_be(bytes, offset).map(Oid)
    }

    /// Read 4-byte price at offset.
    #[inline]
    pub fn read_price(
        bytes: &[u8],
        offset: usize,
    ) -> Result<WirePrice, FeedError> {
        read_u32_be(bytes, offset).map(WirePrice)
    }

    /// Read 4-byte quantity at offset.
    #[inline]
    pub fn read_qty(bytes: &[u8], offset: usize) -> Result<WireQty, FeedError> {
        read_u32_be(bytes, offset).map(WireQty)
    }

    /// Read 2-byte stock locate at offset.
    #[inline]
    pub fn read_locate(
        bytes: &[u8],
        offset: usize,
    ) -> Result<StockLocate, FeedError> {
        read_u16_be(bytes, offset).map(StockLocate)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn read_primitives_bounds() {
            let b = [1u8, 2, 3, 4, 5, 6, 7, 8];
            assert_eq!(read_u16_be(&b, 0).unwrap(), 0x0102);
            assert_eq!(read_u32_be(&b, 0).unwrap(), 0x01020304);
            assert_eq!(read_u64_be(&b, 0).unwrap(), 0x0102030405060708);
            assert_eq!(read_timestamp(&[0, 0, 0, 0, 0, 1][..], 0).unwrap().0, 1);
            assert!(read_u32_be(&b, 6).is_err());
            assert!(read_timestamp(&b, 3).is_err());
        }

        #[test]
        fn buy_sell_bytes() {
            assert_eq!(BuySell::try_from(b'B').unwrap() as u8, b'B');
            assert_eq!(BuySell::try_from(b'S').unwrap() as u8, b'S');
            assert!(BuySell::try_from(0).is_err());
        }

        #[test]
        fn read_typed_helpers() {
            let mut b = [0u8; 16];
            b[0..2].copy_from_slice(&0x0304u16.to_be_bytes());
            assert_eq!(read_locate(&b, 0).unwrap().0, 0x0304);

            b[0..8].copy_from_slice(&0x1122_3344_5566_7788u64.to_be_bytes());
            assert_eq!(read_oid(&b, 0).unwrap().0, 0x1122_3344_5566_7788);

            b[0..4].copy_from_slice(&0xdead_beefu32.to_be_bytes());
            assert_eq!(read_price(&b, 0).unwrap().0, 0xdead_beef);
            assert_eq!(read_qty(&b, 0).unwrap().0, 0xdead_beef);

            assert!(read_oid(&[0u8; 7], 0).is_err());
            assert!(read_price(&[0u8; 3], 0).is_err());
        }
    }
}

use self::wire::{
    BuySell, Oid, StockLocate, Timestamp, WirePrice, WireQty, read_locate,
    read_oid, read_price, read_qty, read_timestamp,
};
use crate::error::FeedError;

#[inline]
fn require_len(bytes: &[u8], required: usize) -> Result<(), FeedError> {
    if bytes.len() < required {
        return Err(FeedError::Parse {
            required,
            got: bytes.len(),
        });
    }
    Ok(())
}

/// Fixed-length ITCH payload: length check then decode (used by [`parse_fixed`]).
pub trait ItchFixedPayload: Sized {
    const MIN_LEN: usize;
    fn decode_body(bytes: &[u8]) -> Result<Self, FeedError>;
}

#[inline]
fn parse_fixed<T: ItchFixedPayload>(bytes: &[u8]) -> Result<T, FeedError> {
    require_len(bytes, T::MIN_LEN)?;
    T::decode_body(bytes)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ItchMsgType {
    Sysevent = b'S',
    StockDirectory = b'R',
    TradingAction = b'H',
    RegShoRestrict = b'Y',
    MpidPosition = b'L',
    MwcbDecline = b'V',
    MwcbStatus = b'W',
    IpoQuoteUpdate = b'K',
    AddOrder = b'A',
    AddOrderMpid = b'F',
    ExecuteOrder = b'E',
    ExecuteOrderWithPrice = b'C',
    ReduceOrder = b'X',
    DeleteOrder = b'D',
    ReplaceOrder = b'U',
    Trade = b'P',
    CrossTrade = b'Q',
    BrokenTrade = b'B',
    NetOrderImbalance = b'I',
    RetailPriceImprovement = b'N',
    ProcessLuldAuctionCollarMessage = b'J',
}

impl TryFrom<u8> for ItchMsgType {
    type Error = FeedError;
    fn try_from(b: u8) -> Result<Self, FeedError> {
        use ItchMsgType::*;
        match b {
            b'S' => Ok(Sysevent),
            b'R' => Ok(StockDirectory),
            b'H' => Ok(TradingAction),
            b'Y' => Ok(RegShoRestrict),
            b'L' => Ok(MpidPosition),
            b'V' => Ok(MwcbDecline),
            b'W' => Ok(MwcbStatus),
            b'K' => Ok(IpoQuoteUpdate),
            b'A' => Ok(AddOrder),
            b'F' => Ok(AddOrderMpid),
            b'E' => Ok(ExecuteOrder),
            b'C' => Ok(ExecuteOrderWithPrice),
            b'X' => Ok(ReduceOrder),
            b'D' => Ok(DeleteOrder),
            b'U' => Ok(ReplaceOrder),
            b'P' => Ok(Trade),
            b'Q' => Ok(CrossTrade),
            b'B' => Ok(BrokenTrade),
            b'I' => Ok(NetOrderImbalance),
            b'N' => Ok(RetailPriceImprovement),
            b'J' => Ok(ProcessLuldAuctionCollarMessage),
            _ => Err(FeedError::InvalidMessageType(b)),
        }
    }
}

#[inline]
pub fn message_len(t: ItchMsgType) -> u8 {
    use ItchMsgType::*;
    match t {
        Sysevent => 12,
        StockDirectory => 39,
        TradingAction => 25,
        RegShoRestrict => 20,
        MpidPosition => 26,
        MwcbDecline => 35,
        MwcbStatus => 12,
        IpoQuoteUpdate => 28,
        AddOrder => 36,
        AddOrderMpid => 40,
        ExecuteOrder => 31,
        ExecuteOrderWithPrice => 36,
        ReduceOrder => 23,
        DeleteOrder => 19,
        ReplaceOrder => 35,
        Trade => 44,
        CrossTrade => 40,
        BrokenTrade => 19,
        NetOrderImbalance => 50,
        RetailPriceImprovement => 20,
        ProcessLuldAuctionCollarMessage => 35,
    }
}

#[derive(Debug, Clone)]
pub struct AddOrder {
    pub timestamp: Timestamp,
    pub oid: Oid,
    pub price: WirePrice,
    pub qty: WireQty,
    pub stock_locate: StockLocate,
    pub buy: BuySell,
}

impl ItchFixedPayload for AddOrder {
    const MIN_LEN: usize = 36;
    fn decode_body(bytes: &[u8]) -> Result<Self, FeedError> {
        let buy = BuySell::try_from(bytes[19])
            .map_err(|_| FeedError::InvalidBuySell(bytes[19]))?;
        Ok(AddOrder {
            stock_locate: read_locate(bytes, 1)?,
            timestamp: read_timestamp(bytes, 5)?,
            oid: read_oid(bytes, 11)?,
            buy,
            qty: read_qty(bytes, 20)?,
            price: read_price(bytes, 32)?,
        })
    }
}

impl AddOrder {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        parse_fixed(bytes)
    }
}

#[derive(Debug, Clone)]
pub struct AddOrderMpid {
    pub add_msg: AddOrder,
}

impl AddOrderMpid {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        require_len(bytes, 40)?;
        Ok(AddOrderMpid {
            add_msg: AddOrder::parse(bytes)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ExecuteOrder {
    pub oid: Oid,
    #[allow(dead_code)]
    pub timestamp: Timestamp,
    pub qty: WireQty,
    #[allow(dead_code)]
    pub stock_locate: StockLocate,
}

impl ItchFixedPayload for ExecuteOrder {
    const MIN_LEN: usize = 31;
    fn decode_body(bytes: &[u8]) -> Result<Self, FeedError> {
        Ok(ExecuteOrder {
            stock_locate: read_locate(bytes, 1)?,
            timestamp: read_timestamp(bytes, 5)?,
            oid: read_oid(bytes, 11)?,
            qty: read_qty(bytes, 19)?,
        })
    }
}

impl ExecuteOrder {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        parse_fixed(bytes)
    }
}

#[derive(Debug, Clone)]
pub struct ExecuteOrderWithPrice {
    pub exec: ExecuteOrder,
}

impl ExecuteOrderWithPrice {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        require_len(bytes, 36)?;
        Ok(ExecuteOrderWithPrice {
            exec: ExecuteOrder::parse(bytes)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ReduceOrder {
    pub oid: Oid,
    #[allow(dead_code)]
    pub timestamp: Timestamp,
    pub qty: WireQty,
}

impl ItchFixedPayload for ReduceOrder {
    const MIN_LEN: usize = 23;
    fn decode_body(bytes: &[u8]) -> Result<Self, FeedError> {
        Ok(ReduceOrder {
            timestamp: read_timestamp(bytes, 5)?,
            oid: read_oid(bytes, 11)?,
            qty: read_qty(bytes, 19)?,
        })
    }
}

impl ReduceOrder {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        parse_fixed(bytes)
    }
}

#[derive(Debug, Clone)]
pub struct DeleteOrder {
    pub oid: Oid,
    #[allow(dead_code)]
    pub timestamp: Timestamp,
}

impl ItchFixedPayload for DeleteOrder {
    const MIN_LEN: usize = 19;
    fn decode_body(bytes: &[u8]) -> Result<Self, FeedError> {
        Ok(DeleteOrder {
            timestamp: read_timestamp(bytes, 5)?,
            oid: read_oid(bytes, 11)?,
        })
    }
}

impl DeleteOrder {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        parse_fixed(bytes)
    }
}

#[derive(Debug, Clone)]
pub struct ReplaceOrder {
    pub oid: Oid,
    pub new_order_id: Oid,
    pub new_qty: WireQty,
    pub new_price: WirePrice,
}

impl ItchFixedPayload for ReplaceOrder {
    const MIN_LEN: usize = 35;
    fn decode_body(bytes: &[u8]) -> Result<Self, FeedError> {
        Ok(ReplaceOrder {
            oid: read_oid(bytes, 11)?,
            new_order_id: read_oid(bytes, 19)?,
            new_qty: read_qty(bytes, 27)?,
            new_price: read_price(bytes, 31)?,
        })
    }
}

impl ReplaceOrder {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        parse_fixed(bytes)
    }
}

pub mod stream {
    //! Process ITCH stream into order commands and apply via the engine.
    #![allow(missing_docs)]

    use std::io::Read;

    use super::wire::BuySell;
    use super::{
        AddOrder as ItchAddOrder, AddOrderMpid, DeleteOrder, ExecuteOrder,
        ExecuteOrderWithPrice, ItchMsgType, ReduceOrder, ReplaceOrder,
        message_len,
    };
    use crate::engine::{
        AddOrderCommand, CancelByOrderIdCommand, ExecuteOrderCommand,
        ReduceOrderCommand, ReplaceOrderByNewIdCommand,
    };
    use crate::engine::{OrderCommand, OrderMatchingService};
    use crate::error::Error;
    use crate::error::{FeedError, Result};
    use crate::itch::buf::BufferedReader;
    use crate::types::{OrderType, ParticipantId, Side, SymbolId, TimeInForce};

    /// Result of decoding one ITCH book message (optional command or parse error).
    pub type DecodeResult = std::result::Result<Option<OrderCommand>, FeedError>;

    /// Parsed ITCH frame header: message kind and total byte length (2-byte len + type + payload).
    pub type ItchFrameHeader = (ItchMsgType, usize);

    #[inline]
    fn is_book_affecting(msg_type: ItchMsgType) -> bool {
        matches!(
            msg_type,
            ItchMsgType::AddOrder
                | ItchMsgType::AddOrderMpid
                | ItchMsgType::ExecuteOrder
                | ItchMsgType::ExecuteOrderWithPrice
                | ItchMsgType::ReduceOrder
                | ItchMsgType::DeleteOrder
                | ItchMsgType::ReplaceOrder
        )
    }

    /// Read the 2-byte length + 1-byte message type and validate `wire_len` vs fixed ITCH size.
    #[inline]
    pub(crate) fn read_itch_header(
        data: &[u8],
    ) -> std::result::Result<ItchFrameHeader, FeedError> {
        if data.len() < 3 {
            return Err(FeedError::Parse {
                required: 3,
                got: data.len(),
            });
        }
        let wire_len = u16::from_be_bytes([data[0], data[1]]) as usize;
        let msg_type = ItchMsgType::try_from(data[2])?;
        let payload_len = message_len(msg_type) as usize;
        if wire_len != payload_len {
            return Err(FeedError::Parse {
                required: payload_len,
                got: wire_len,
            });
        }
        Ok((msg_type, 3 + payload_len))
    }

    fn apply_itch_payload<E: OrderMatchingService>(
        msg_type: ItchMsgType,
        payload: &[u8],
        engine: &mut E,
        npkts: &mut u64,
    ) -> Result<()> {
        if is_book_affecting(msg_type) {
            *npkts += 1;
            if let Some(cmd) =
                decode_book_message(msg_type, payload).map_err(Error::from)?
            {
                engine.process(cmd)?;
            }
        }
        Ok(())
    }

    /// Process a contiguous byte slice of ITCH packets (each: u16 length BE, u8 type, payload).
    /// Used for tests, replay buffers, and decode throughput benches (avoids `Read` overhead).
    /// Returns the number of book-affecting messages processed.
    pub fn process_itch_bytes<E: OrderMatchingService>(
        data: &[u8],
        engine: &mut E,
    ) -> Result<u64> {
        let mut i = 0usize;
        let mut npkts: u64 = 0;
        while i < data.len() {
            let rest = &data[i..];
            let (msg_type, total) =
                read_itch_header(rest).map_err(Error::from)?;
            if rest.len() < total {
                return Err(FeedError::Parse {
                    required: total,
                    got: rest.len(),
                }
                .into());
            }
            let payload = &rest[3..total];
            apply_itch_payload(msg_type, payload, engine, &mut npkts)?;
            i += total;
        }
        Ok(npkts)
    }

    /// Walk ITCH packets and fully decode every book-affecting payload (no engine).
    /// For benchmarking parse throughput and for regressions on the decode hot path.
    pub fn scan_decode_book_messages(
        data: &[u8],
    ) -> std::result::Result<u64, FeedError> {
        let mut i = 0usize;
        let mut book_count: u64 = 0;
        while i < data.len() {
            let rest = &data[i..];
            let (msg_type, total) = read_itch_header(rest)?;
            if rest.len() < total {
                return Err(FeedError::Parse {
                    required: total,
                    got: rest.len(),
                });
            }
            let payload = &rest[3..total];
            if is_book_affecting(msg_type) {
                book_count += 1;
                decode_book_message(msg_type, payload)?;
            }
            i += total;
        }
        Ok(book_count)
    }

    #[inline]
    fn itch_limit_add(
        oid: u64,
        stock_locate: u16,
        buy: BuySell,
        qty: u32,
        price: u32,
    ) -> OrderCommand {
        const ITCH_PARTICIPANT: ParticipantId = 0;
        OrderCommand::Add(AddOrderCommand {
            id: oid,
            participant_id: ITCH_PARTICIPANT,
            symbol_id: SymbolId::from(stock_locate),
            side: if matches!(buy, BuySell::Buy) {
                Side::Buy
            } else {
                Side::Sell
            },
            order_type: OrderType::Limit,
            price: Some(price as i64),
            quantity: qty as i64,
            time_in_force: TimeInForce::Gtc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
        })
    }

    #[inline]
    fn itch_execute(oid: u64, qty: u32) -> OrderCommand {
        OrderCommand::Execute(ExecuteOrderCommand {
            order_id: oid,
            quantity: qty as i64,
        })
    }

    #[inline]
    fn itch_reduce(oid: u64, qty: u32) -> OrderCommand {
        OrderCommand::Reduce(ReduceOrderCommand {
            order_id: oid,
            quantity: qty as i64,
        })
    }

    #[inline]
    fn itch_cancel(oid: u64) -> OrderCommand {
        OrderCommand::CancelByOrderId(CancelByOrderIdCommand { order_id: oid })
    }

    #[inline]
    fn itch_replace(
        old_oid: u64,
        new_oid: u64,
        new_price: u32,
        new_qty: u32,
    ) -> OrderCommand {
        OrderCommand::ReplaceByNewId(ReplaceOrderByNewIdCommand {
            old_order_id: old_oid,
            new_order_id: new_oid,
            new_price: new_price as i64,
            new_quantity: new_qty as i64,
            symbol_id: None,
            side: None,
        })
    }

    /// Decode one book-affecting ITCH message into an [`OrderCommand`].
    /// Returns `Ok(None)` for non–book-affecting or skipped messages.
    pub fn decode_book_message(
        msg_type: ItchMsgType,
        payload: &[u8],
    ) -> DecodeResult {
        let cmd = match msg_type {
            ItchMsgType::AddOrder => {
                let m = ItchAddOrder::parse(payload)?;
                Some(itch_limit_add(
                    m.oid.0,
                    m.stock_locate.0,
                    m.buy,
                    m.qty.0,
                    m.price.0,
                ))
            }
            ItchMsgType::AddOrderMpid => {
                let m = AddOrderMpid::parse(payload)?;
                let a = &m.add_msg;
                Some(itch_limit_add(
                    a.oid.0,
                    a.stock_locate.0,
                    a.buy,
                    a.qty.0,
                    a.price.0,
                ))
            }
            ItchMsgType::ExecuteOrder => {
                let m = ExecuteOrder::parse(payload)?;
                Some(itch_execute(m.oid.0, m.qty.0))
            }
            ItchMsgType::ExecuteOrderWithPrice => {
                let m = ExecuteOrderWithPrice::parse(payload)?;
                Some(itch_execute(m.exec.oid.0, m.exec.qty.0))
            }
            ItchMsgType::ReduceOrder => {
                let m = ReduceOrder::parse(payload)?;
                Some(itch_reduce(m.oid.0, m.qty.0))
            }
            ItchMsgType::DeleteOrder => {
                let m = DeleteOrder::parse(payload)?;
                Some(itch_cancel(m.oid.0))
            }
            ItchMsgType::ReplaceOrder => {
                let m = ReplaceOrder::parse(payload)?;
                Some(itch_replace(
                    m.oid.0,
                    m.new_order_id.0,
                    m.new_price.0,
                    m.new_qty.0,
                ))
            }
            _ => None,
        };
        Ok(cmd)
    }

    /// Process an ITCH 5.0 stream from `reader`, decode book-affecting messages into
    /// [`OrderCommand`]s, and apply them via the engine.
    /// Returns the number of book-affecting messages processed, or an error.
    pub fn process_itch_stream<R: Read, E: OrderMatchingService>(
        reader: R,
        engine: &mut E,
    ) -> Result<u64> {
        let mut buf = BufferedReader::new(1024, reader);
        let mut npkts: u64 = 0;

        while buf.ensure(3).is_ok() {
            let slice = buf.get(0);
            let (msg_type, total) =
                read_itch_header(slice).map_err(Error::from)?;
            buf.ensure(total)?;
            let payload = &buf.get(0)[3..total];
            apply_itch_payload(msg_type, payload, engine, &mut npkts)?;
            buf.advance(total);
        }

        Ok(npkts)
    }

    #[cfg(test)]
    mod tests {
        use std::io::Cursor;

        use crate::book::service::BTreeOrderBook;
        use crate::engine::{OrderCommand, OrderMatchingEngine};
        use crate::error::FeedError;
        use crate::events::NoOpEventSink;
        use crate::matching::PriceCrossMatchingPolicy;
        use crate::self_trade::AllowAllSelfTradePolicy;
        use crate::sequence::CounterSequenceGenerator;
        use crate::store::service::HashMapOrderStore;

        use super::super::{ItchMsgType, message_len};
        use super::{
            decode_book_message, process_itch_bytes, process_itch_stream,
            read_itch_header, scan_decode_book_messages,
        };

        /// One ITCH AddOrder packet: len=36, type='A', payload with oid=1, buy, qty=10, price=10000, stock_locate=0.
        fn one_add_order_itch_packet() -> Vec<u8> {
            let mut buf = vec![0u8; 3 + 36];
            buf[0..2].copy_from_slice(&36u16.to_be_bytes());
            buf[2] = b'A';
            buf[4..6].copy_from_slice(&0u16.to_be_bytes());
            buf[8..14].copy_from_slice(&[0u8; 6]);
            buf[14..22].copy_from_slice(&1u64.to_be_bytes());
            buf[22] = b'B';
            buf[23..27].copy_from_slice(&10u32.to_be_bytes());
            buf[35..39].copy_from_slice(&10000u32.to_be_bytes());
            buf
        }

        #[allow(clippy::type_complexity)]
        fn engine() -> OrderMatchingEngine<
            CounterSequenceGenerator,
            BTreeOrderBook,
            HashMapOrderStore,
            PriceCrossMatchingPolicy,
            AllowAllSelfTradePolicy,
            NoOpEventSink,
        > {
            let seq = CounterSequenceGenerator::new();
            let book = BTreeOrderBook::new();
            let store = HashMapOrderStore::new();
            let matching = PriceCrossMatchingPolicy;
            let self_trade = AllowAllSelfTradePolicy;
            let sink = NoOpEventSink;
            OrderMatchingEngine::new(seq, book, store, matching, self_trade, sink)
        }

        #[test]
        fn process_itch_stream_applies_to_engine() {
            let mut engine = engine();
            let data = one_add_order_itch_packet();
            let n = process_itch_stream(Cursor::new(&data), &mut engine).unwrap();
            assert_eq!(n, 1);
            assert_eq!(engine.best_bid(), Some(10000));
            assert!(engine.best_ask().is_none());
        }

        #[test]
        fn process_itch_bytes_matches_stream() {
            let data = one_add_order_itch_packet();
            let mut e1 = engine();
            let mut e2 = engine();
            let n1 = process_itch_stream(Cursor::new(&data), &mut e1).unwrap();
            let n2 = process_itch_bytes(&data, &mut e2).unwrap();
            assert_eq!(n1, n2);
            assert_eq!(e1.best_bid(), e2.best_bid());
        }

        #[test]
        fn read_itch_header_rejects_len_mismatch() {
            let mut bad = vec![0u8; 3];
            bad[0..2].copy_from_slice(&99u16.to_be_bytes());
            bad[2] = b'A';
            assert!(matches!(
                read_itch_header(&bad),
                Err(FeedError::Parse { .. })
            ));
        }

        #[test]
        fn scan_decode_skips_non_book_messages() {
            let mut buf = Vec::new();
            let stock_dir_len = message_len(ItchMsgType::StockDirectory) as usize;
            let mut sd = vec![0u8; 3 + stock_dir_len];
            sd[0..2].copy_from_slice(&(stock_dir_len as u16).to_be_bytes());
            sd[2] = b'R';
            buf.extend_from_slice(&sd);
            buf.extend_from_slice(&one_add_order_itch_packet());
            assert_eq!(scan_decode_book_messages(&buf).unwrap(), 1);
        }

        #[test]
        fn decode_book_message_variants() {
            let mut add = vec![0u8; 36];
            add[1..3].copy_from_slice(&0u16.to_be_bytes());
            add[11..19].copy_from_slice(&10u64.to_be_bytes());
            add[19] = b'B';
            add[20..24].copy_from_slice(&2u32.to_be_bytes());
            add[32..36].copy_from_slice(&100u32.to_be_bytes());
            assert!(matches!(
                decode_book_message(ItchMsgType::AddOrder, &add).unwrap(),
                Some(OrderCommand::Add(_))
            ));

            let mut del = vec![0u8; 19];
            del[11..19].copy_from_slice(&10u64.to_be_bytes());
            assert!(matches!(
                decode_book_message(ItchMsgType::DeleteOrder, &del).unwrap(),
                Some(OrderCommand::CancelByOrderId(_))
            ));

            let mut red = vec![0u8; 23];
            red[11..19].copy_from_slice(&10u64.to_be_bytes());
            red[19..23].copy_from_slice(&1u32.to_be_bytes());
            assert!(matches!(
                decode_book_message(ItchMsgType::ReduceOrder, &red).unwrap(),
                Some(OrderCommand::Reduce(_))
            ));

            let mut ex = vec![0u8; 31];
            ex[11..19].copy_from_slice(&3u64.to_be_bytes());
            ex[19..23].copy_from_slice(&4u32.to_be_bytes());
            assert!(matches!(
                decode_book_message(ItchMsgType::ExecuteOrder, &ex).unwrap(),
                Some(OrderCommand::Execute(_))
            ));

            let mut rep = vec![0u8; 35];
            rep[11..19].copy_from_slice(&1u64.to_be_bytes());
            rep[19..27].copy_from_slice(&2u64.to_be_bytes());
            rep[27..31].copy_from_slice(&5u32.to_be_bytes());
            rep[31..35].copy_from_slice(&200u32.to_be_bytes());
            assert!(matches!(
                decode_book_message(ItchMsgType::ReplaceOrder, &rep).unwrap(),
                Some(OrderCommand::ReplaceByNewId(_))
            ));

            let mut fmpid = vec![0u8; 40];
            fmpid[1..3].copy_from_slice(&0u16.to_be_bytes());
            fmpid[11..19].copy_from_slice(&7u64.to_be_bytes());
            fmpid[19] = b'S';
            fmpid[20..24].copy_from_slice(&3u32.to_be_bytes());
            fmpid[32..36].copy_from_slice(&50u32.to_be_bytes());
            fmpid[36..40].copy_from_slice(&0x41424344u32.to_be_bytes());
            assert!(matches!(
                decode_book_message(ItchMsgType::AddOrderMpid, &fmpid).unwrap(),
                Some(OrderCommand::Add(_))
            ));

            let mut ewp = vec![0u8; 36];
            ewp[11..19].copy_from_slice(&8u64.to_be_bytes());
            ewp[19..23].copy_from_slice(&6u32.to_be_bytes());
            assert!(matches!(
                decode_book_message(ItchMsgType::ExecuteOrderWithPrice, &ewp)
                    .unwrap(),
                Some(OrderCommand::Execute(_))
            ));

            assert!(
                decode_book_message(ItchMsgType::Trade, &[0u8; 44])
                    .unwrap()
                    .is_none()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn itch_msg_type_try_from_matches_message_len_table() {
        let all = [
            ItchMsgType::Sysevent,
            ItchMsgType::StockDirectory,
            ItchMsgType::TradingAction,
            ItchMsgType::RegShoRestrict,
            ItchMsgType::MpidPosition,
            ItchMsgType::MwcbDecline,
            ItchMsgType::MwcbStatus,
            ItchMsgType::IpoQuoteUpdate,
            ItchMsgType::AddOrder,
            ItchMsgType::AddOrderMpid,
            ItchMsgType::ExecuteOrder,
            ItchMsgType::ExecuteOrderWithPrice,
            ItchMsgType::ReduceOrder,
            ItchMsgType::DeleteOrder,
            ItchMsgType::ReplaceOrder,
            ItchMsgType::Trade,
            ItchMsgType::CrossTrade,
            ItchMsgType::BrokenTrade,
            ItchMsgType::NetOrderImbalance,
            ItchMsgType::RetailPriceImprovement,
            ItchMsgType::ProcessLuldAuctionCollarMessage,
        ];
        for t in all {
            let b = t as u8;
            assert_eq!(ItchMsgType::try_from(b).unwrap() as u8, b);
            assert!(message_len(t) > 0);
        }
    }

    #[test]
    fn unknown_message_type_rejected() {
        assert!(matches!(
            ItchMsgType::try_from(0x7f),
            Err(FeedError::InvalidMessageType(0x7f))
        ));
    }

    #[test]
    fn add_order_roundtrip_and_invalid_buy_sell() {
        let mut p = vec![0u8; 36];
        p[1..3].copy_from_slice(&7u16.to_be_bytes());
        p[11..19].copy_from_slice(&42u64.to_be_bytes());
        p[19] = b'B';
        p[20..24].copy_from_slice(&100u32.to_be_bytes());
        p[32..36].copy_from_slice(&50_000u32.to_be_bytes());
        let m = AddOrder::parse(&p).unwrap();
        assert_eq!(m.stock_locate.0, 7);
        assert_eq!(m.oid.0, 42);
        assert_eq!(m.qty.0, 100);
        assert_eq!(m.price.0, 50_000);
        assert!(matches!(m.buy, BuySell::Buy));

        p[19] = b'X';
        assert!(matches!(
            AddOrder::parse(&p),
            Err(FeedError::InvalidBuySell(b'X'))
        ));

        assert!(matches!(
            AddOrder::parse(&p[..10]),
            Err(FeedError::Parse { .. })
        ));
    }

    #[test]
    fn add_order_mpid_requires_40_bytes() {
        let mut p = vec![0u8; 40];
        p[1..3].copy_from_slice(&1u16.to_be_bytes());
        p[11..19].copy_from_slice(&9u64.to_be_bytes());
        p[19] = b'S';
        p[20..24].copy_from_slice(&1u32.to_be_bytes());
        p[32..36].copy_from_slice(&123u32.to_be_bytes());
        assert!(AddOrderMpid::parse(&p[..39]).is_err());
        let _ = AddOrderMpid::parse(&p).unwrap();
    }

    #[test]
    fn delete_reduce_execute_replace_short_buffer() {
        assert!(DeleteOrder::parse(&[0u8; 10]).is_err());
        assert!(ReduceOrder::parse(&[0u8; 10]).is_err());
        assert!(ExecuteOrder::parse(&[0u8; 10]).is_err());
        assert!(ExecuteOrderWithPrice::parse(&[0u8; 35]).is_err());
        assert!(ReplaceOrder::parse(&[0u8; 10]).is_err());
    }

    #[test]
    fn execute_with_price_accepts_36_bytes() {
        let mut p = vec![0u8; 36];
        p[11..19].copy_from_slice(&99u64.to_be_bytes());
        p[19..23].copy_from_slice(&5u32.to_be_bytes());
        let m = ExecuteOrderWithPrice::parse(&p).unwrap();
        assert_eq!(m.exec.oid.0, 99);
        assert_eq!(m.exec.qty.0, 5);
    }

    #[test]
    fn replace_order_fields() {
        let mut p = vec![0u8; 35];
        p[11..19].copy_from_slice(&1u64.to_be_bytes());
        p[19..27].copy_from_slice(&2u64.to_be_bytes());
        p[27..31].copy_from_slice(&30u32.to_be_bytes());
        p[31..35].copy_from_slice(&40_000u32.to_be_bytes());
        let m = ReplaceOrder::parse(&p).unwrap();
        assert_eq!(m.oid.0, 1);
        assert_eq!(m.new_order_id.0, 2);
        assert_eq!(m.new_qty.0, 30);
        assert_eq!(m.new_price.0, 40_000);
    }
}
