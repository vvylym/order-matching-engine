//! Process ITCH stream into order commands and apply via the engine.

use std::io::Read;

use crate::engine::{
    AddOrderCommand, CancelByOrderIdCommand, ExecuteOrderCommand,
    ReduceOrderCommand, ReplaceOrderByNewIdCommand,
};
use crate::engine::{OrderCommand, OrderMatchingService};
use crate::error::Error;
use crate::error::{FeedError, Result};
use crate::itch::buf::BufferedReader;
use crate::itch::messages::{
    AddOrder as ItchAddOrder, AddOrderMpid, DeleteOrder, ExecuteOrder,
    ExecuteOrderWithPrice, ItchMsgType, ReduceOrder, ReplaceOrder, message_len,
};
use crate::itch::wire::BuySell;
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
        let (msg_type, total) = read_itch_header(rest).map_err(Error::from)?;
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

/// Decode one book-affecting ITCH message into an [`OrderCommand`].
/// Returns `Ok(None)` for non–book-affecting or skipped messages.
pub fn decode_book_message(
    msg_type: ItchMsgType,
    payload: &[u8],
) -> DecodeResult {
    const ITCH_PARTICIPANT: ParticipantId = 0;
    let cmd = match msg_type {
        ItchMsgType::AddOrder => {
            let m = ItchAddOrder::parse(payload)?;
            Some(OrderCommand::Add(AddOrderCommand {
                id: m.oid.0,
                participant_id: ITCH_PARTICIPANT,
                symbol_id: SymbolId::from(m.stock_locate.0),
                side: if matches!(m.buy, BuySell::Buy) {
                    Side::Buy
                } else {
                    Side::Sell
                },
                order_type: OrderType::Limit,
                price: Some(m.price.0 as i64),
                quantity: m.qty.0 as i64,
                time_in_force: TimeInForce::Gtc,
                stop_price: None,
                max_visible_quantity: None,
                slippage: None,
                trailing_distance: None,
                trailing_step: None,
            }))
        }
        ItchMsgType::AddOrderMpid => {
            let m = AddOrderMpid::parse(payload)?;
            let a = &m.add_msg;
            Some(OrderCommand::Add(AddOrderCommand {
                id: a.oid.0,
                participant_id: ITCH_PARTICIPANT,
                symbol_id: SymbolId::from(a.stock_locate.0),
                side: if matches!(a.buy, BuySell::Buy) {
                    Side::Buy
                } else {
                    Side::Sell
                },
                order_type: OrderType::Limit,
                price: Some(a.price.0 as i64),
                quantity: a.qty.0 as i64,
                time_in_force: TimeInForce::Gtc,
                stop_price: None,
                max_visible_quantity: None,
                slippage: None,
                trailing_distance: None,
                trailing_step: None,
            }))
        }
        ItchMsgType::ExecuteOrder => {
            let m = ExecuteOrder::parse(payload)?;
            Some(OrderCommand::Execute(ExecuteOrderCommand {
                order_id: m.oid.0,
                quantity: m.qty.0 as i64,
            }))
        }
        ItchMsgType::ExecuteOrderWithPrice => {
            let m = ExecuteOrderWithPrice::parse(payload)?;
            Some(OrderCommand::Execute(ExecuteOrderCommand {
                order_id: m.exec.oid.0,
                quantity: m.exec.qty.0 as i64,
            }))
        }
        ItchMsgType::ReduceOrder => {
            let m = ReduceOrder::parse(payload)?;
            Some(OrderCommand::Reduce(ReduceOrderCommand {
                order_id: m.oid.0,
                quantity: m.qty.0 as i64,
            }))
        }
        ItchMsgType::DeleteOrder => {
            let m = DeleteOrder::parse(payload)?;
            Some(OrderCommand::CancelByOrderId(CancelByOrderIdCommand {
                order_id: m.oid.0,
            }))
        }
        ItchMsgType::ReplaceOrder => {
            let m = ReplaceOrder::parse(payload)?;
            Some(OrderCommand::ReplaceByNewId(ReplaceOrderByNewIdCommand {
                old_order_id: m.oid.0,
                new_order_id: m.new_order_id.0,
                new_price: m.new_price.0 as i64,
                new_quantity: m.new_qty.0 as i64,
                symbol_id: None,
                side: None,
            }))
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
        let (msg_type, total) = read_itch_header(slice).map_err(Error::from)?;
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
    use crate::itch::message_len;
    use crate::itch::messages::ItchMsgType;
    use crate::matching::PriceCrossMatchingPolicy;
    use crate::self_trade::AllowAllSelfTradePolicy;
    use crate::sequence::CounterSequenceGenerator;
    use crate::store::service::HashMapOrderStore;

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

        assert!(
            decode_book_message(ItchMsgType::Trade, &[0u8; 44])
                .unwrap()
                .is_none()
        );
    }
}
