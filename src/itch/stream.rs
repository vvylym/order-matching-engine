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
        let wire_len = u16::from_be_bytes([slice[0], slice[1]]) as usize;
        let msg_type = ItchMsgType::try_from(slice[2]).map_err(Error::from)?;
        let payload_len = message_len(msg_type) as usize;
        if wire_len != payload_len {
            return Err(FeedError::Parse {
                required: payload_len,
                got: wire_len,
            }
            .into());
        }
        buf.ensure(3 + payload_len)?;
        let payload = &buf.get(0)[3..3 + payload_len];

        if is_book_affecting(msg_type) {
            npkts += 1;
            if let Some(cmd) =
                decode_book_message(msg_type, payload).map_err(Error::from)?
            {
                engine.process(cmd)?;
            }
        }

        buf.advance(3 + payload_len);
    }

    Ok(npkts)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::book::service::BTreeOrderBook;
    use crate::engine::OrderMatchingEngine;
    use crate::events::NoOpEventSink;
    use crate::matching::PriceCrossMatchingPolicy;
    use crate::self_trade::AllowAllSelfTradePolicy;
    use crate::sequence::CounterSequenceGenerator;
    use crate::store::service::HashMapOrderStore;

    use super::process_itch_stream;

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

    #[test]
    fn process_itch_stream_applies_to_engine() {
        let seq = CounterSequenceGenerator::new();
        let book = BTreeOrderBook::new();
        let store = HashMapOrderStore::new();
        let matching = PriceCrossMatchingPolicy;
        let self_trade = AllowAllSelfTradePolicy;
        let sink = NoOpEventSink;
        let mut engine = OrderMatchingEngine::new(
            seq, book, store, matching, self_trade, sink,
        );
        let data = one_add_order_itch_packet();
        let n = process_itch_stream(Cursor::new(&data), &mut engine).unwrap();
        assert_eq!(n, 1);
        assert_eq!(engine.best_bid(), Some(10000));
        assert!(engine.best_ask().is_none());
    }
}
