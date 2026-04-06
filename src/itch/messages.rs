//! ITCH 5.0 message types and payload parsing.
#![allow(missing_docs)]

use crate::error::FeedError;
use crate::itch::wire::{
    BuySell, Oid, StockLocate, Timestamp, WirePrice, WireQty, read_locate,
    read_oid, read_price, read_qty, read_timestamp,
};

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

impl AddOrder {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        const LEN: usize = 36;
        if bytes.len() < LEN {
            return Err(FeedError::Parse {
                required: LEN,
                got: bytes.len(),
            });
        }
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

#[derive(Debug, Clone)]
pub struct AddOrderMpid {
    pub add_msg: AddOrder,
}

impl AddOrderMpid {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        const LEN: usize = 40;
        if bytes.len() < LEN {
            return Err(FeedError::Parse {
                required: LEN,
                got: bytes.len(),
            });
        }
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

impl ExecuteOrder {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        const LEN: usize = 31;
        if bytes.len() < LEN {
            return Err(FeedError::Parse {
                required: LEN,
                got: bytes.len(),
            });
        }
        Ok(ExecuteOrder {
            stock_locate: read_locate(bytes, 1)?,
            timestamp: read_timestamp(bytes, 5)?,
            oid: read_oid(bytes, 11)?,
            qty: read_qty(bytes, 19)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ExecuteOrderWithPrice {
    pub exec: ExecuteOrder,
}

impl ExecuteOrderWithPrice {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        const LEN: usize = 36;
        if bytes.len() < LEN {
            return Err(FeedError::Parse {
                required: LEN,
                got: bytes.len(),
            });
        }
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

impl ReduceOrder {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        const LEN: usize = 23;
        if bytes.len() < LEN {
            return Err(FeedError::Parse {
                required: LEN,
                got: bytes.len(),
            });
        }
        Ok(ReduceOrder {
            timestamp: read_timestamp(bytes, 5)?,
            oid: read_oid(bytes, 11)?,
            qty: read_qty(bytes, 19)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DeleteOrder {
    pub oid: Oid,
    #[allow(dead_code)]
    pub timestamp: Timestamp,
}

impl DeleteOrder {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        const LEN: usize = 19;
        if bytes.len() < LEN {
            return Err(FeedError::Parse {
                required: LEN,
                got: bytes.len(),
            });
        }
        Ok(DeleteOrder {
            timestamp: read_timestamp(bytes, 5)?,
            oid: read_oid(bytes, 11)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ReplaceOrder {
    pub oid: Oid,
    pub new_order_id: Oid,
    pub new_qty: WireQty,
    pub new_price: WirePrice,
}

impl ReplaceOrder {
    pub fn parse(bytes: &[u8]) -> Result<Self, FeedError> {
        const LEN: usize = 35;
        if bytes.len() < LEN {
            return Err(FeedError::Parse {
                required: LEN,
                got: bytes.len(),
            });
        }
        Ok(ReplaceOrder {
            oid: read_oid(bytes, 11)?,
            new_order_id: read_oid(bytes, 19)?,
            new_qty: read_qty(bytes, 27)?,
            new_price: read_price(bytes, 31)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::itch::wire::BuySell;

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
