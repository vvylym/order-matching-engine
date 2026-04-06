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
