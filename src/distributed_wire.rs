//! Shared wire protocol for distributed benchmark binaries.
//!
//! This module defines a typed command representation and a compact text
//! encoding used by both `src/bin/server.rs` and `src/bin/client.rs`.

use std::fmt;

use crate::engine::{AddOrderCommand, CancelByOrderIdCommand};
use crate::types::{OrderType, Side, TimeInForce};

/// Maximum commands accepted in one batch frame.
pub const MAX_BATCH_COMMANDS: usize = 1024;

/// Protocol-level command used on the gateway/client boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum WireCommand {
    /// Limit add command.
    Add {
        /// Order identifier.
        id: u64,
        /// Participant identifier.
        participant_id: u64,
        /// Destination instrument identifier.
        instrument_id: u32,
        /// Order side.
        side: Side,
        /// Limit price.
        price: i64,
        /// Order quantity.
        quantity: i64,
    },
    /// Market order command.
    Market {
        /// Order identifier.
        id: u64,
        /// Participant identifier.
        participant_id: u64,
        /// Destination instrument identifier.
        instrument_id: u32,
        /// Order side.
        side: Side,
        /// Order quantity.
        quantity: i64,
    },
    /// Cancel by order ID.
    CancelById {
        /// Order identifier to cancel.
        order_id: u64,
        /// Destination instrument identifier.
        instrument_id: u32,
    },
}

/// Parsed frame: either a single command or a command batch.
#[derive(Debug, Clone, PartialEq)]
pub struct WireFrame {
    /// Commands in this frame. Never empty.
    pub commands: Vec<WireCommand>,
}

/// Parse/validation errors returned by wire decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireParseError {
    /// Empty frame.
    Empty,
    /// Unknown command shape.
    Unknown,
    /// Invalid numeric token.
    InvalidNumber,
    /// Invalid side token.
    InvalidSide,
    /// Invalid batch shape.
    InvalidBatch,
    /// Batch exceeds configured limit.
    BatchTooLarge,
    /// Instrument ID is invalid.
    InvalidInstrument,
}

impl fmt::Display for WireParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty"),
            Self::Unknown => write!(f, "unknown"),
            Self::InvalidNumber => write!(f, "invalid_number"),
            Self::InvalidSide => write!(f, "invalid_side"),
            Self::InvalidBatch => write!(f, "invalid_batch"),
            Self::BatchTooLarge => write!(f, "batch_too_large"),
            Self::InvalidInstrument => write!(f, "invalid_instrument"),
        }
    }
}

impl std::error::Error for WireParseError {}

/// Converts wire command into internal add/cancel command + instrument route key.
pub enum RoutedCommand {
    /// Routed add.
    Add {
        /// Destination instrument identifier.
        instrument_id: u32,
        /// Engine add command payload.
        add: AddOrderCommand,
    },
    /// Routed cancel.
    CancelById {
        /// Destination instrument identifier.
        instrument_id: u32,
        /// Engine cancel command payload.
        cancel: CancelByOrderIdCommand,
    },
}

/// Parse one protocol line into a frame.
pub fn parse_frame(line: &str) -> Result<WireFrame, WireParseError> {
    let line = line.trim();
    if line.is_empty() {
        return Err(WireParseError::Empty);
    }

    if let Some(rest) = line.strip_prefix("BATCH ") {
        return parse_batch_frame(rest);
    }

    Ok(WireFrame {
        commands: vec![parse_single(line)?],
    })
}

/// Encode a frame for transport.
pub fn encode_frame(frame: &WireFrame) -> Result<String, WireParseError> {
    if frame.commands.is_empty() {
        return Err(WireParseError::InvalidBatch);
    }
    if frame.commands.len() > MAX_BATCH_COMMANDS {
        return Err(WireParseError::BatchTooLarge);
    }

    let mut encoded: Vec<String> =
        frame.commands.iter().map(encode_single).collect();
    if encoded.len() == 1 {
        let mut line = encoded.pop().expect("single command exists");
        line.push('\n');
        return Ok(line);
    }

    Ok(format!("BATCH {}\n", encoded.join("|")))
}

impl WireCommand {
    /// Convert protocol command into internal routed command.
    pub fn into_routed(self) -> RoutedCommand {
        match self {
            Self::Add {
                id,
                participant_id,
                instrument_id,
                side,
                price,
                quantity,
            } => RoutedCommand::Add {
                instrument_id,
                add: AddOrderCommand {
                    id,
                    participant_id,
                    symbol_id: instrument_id,
                    side,
                    order_type: OrderType::Limit,
                    price: Some(price),
                    quantity,
                    time_in_force: TimeInForce::Gtc,
                    stop_price: None,
                    max_visible_quantity: None,
                    slippage: None,
                    trailing_distance: None,
                    trailing_step: None,
                },
            },
            Self::Market {
                id,
                participant_id,
                instrument_id,
                side,
                quantity,
            } => RoutedCommand::Add {
                instrument_id,
                add: AddOrderCommand {
                    id,
                    participant_id,
                    symbol_id: instrument_id,
                    side,
                    order_type: OrderType::Market,
                    price: None,
                    quantity,
                    time_in_force: TimeInForce::Ioc,
                    stop_price: None,
                    max_visible_quantity: None,
                    slippage: None,
                    trailing_distance: None,
                    trailing_step: None,
                },
            },
            Self::CancelById {
                order_id,
                instrument_id,
            } => RoutedCommand::CancelById {
                instrument_id,
                cancel: CancelByOrderIdCommand { order_id },
            },
        }
    }
}

fn parse_single(line: &str) -> Result<WireCommand, WireParseError> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return Err(WireParseError::Empty);
    }

    match parts[0] {
        "ADD" if parts.len() == 7 => parse_add(parts.as_slice()),
        "MARKET" if parts.len() == 6 => parse_market(parts.as_slice()),
        "CANCELID" if parts.len() == 3 => parse_cancel(parts.as_slice()),
        _ => Err(WireParseError::Unknown),
    }
}

fn parse_batch_frame(rest: &str) -> Result<WireFrame, WireParseError> {
    let raw_cmds: Vec<&str> = rest.split('|').collect();
    if raw_cmds.is_empty() || raw_cmds.iter().any(|cmd| cmd.trim().is_empty()) {
        return Err(WireParseError::InvalidBatch);
    }
    if raw_cmds.len() > MAX_BATCH_COMMANDS {
        return Err(WireParseError::BatchTooLarge);
    }
    let mut commands = Vec::with_capacity(raw_cmds.len());
    for raw in raw_cmds {
        commands.push(parse_single(raw.trim())?);
    }
    Ok(WireFrame { commands })
}

fn parse_add(parts: &[&str]) -> Result<WireCommand, WireParseError> {
    let id = parse_u64(parts[1])?;
    let participant_id = parse_u64(parts[2])?;
    let instrument_id = parse_u32(parts[3])?;
    let side = parse_side(parts[4])?;
    let price = parse_i64(parts[5])?;
    let quantity = parse_i64(parts[6])?;
    validate_instrument(instrument_id)?;
    Ok(WireCommand::Add {
        id,
        participant_id,
        instrument_id,
        side,
        price,
        quantity,
    })
}

fn parse_market(parts: &[&str]) -> Result<WireCommand, WireParseError> {
    let id = parse_u64(parts[1])?;
    let participant_id = parse_u64(parts[2])?;
    let instrument_id = parse_u32(parts[3])?;
    let side = parse_side(parts[4])?;
    let quantity = parse_i64(parts[5])?;
    validate_instrument(instrument_id)?;
    Ok(WireCommand::Market {
        id,
        participant_id,
        instrument_id,
        side,
        quantity,
    })
}

fn parse_cancel(parts: &[&str]) -> Result<WireCommand, WireParseError> {
    let order_id = parse_u64(parts[1])?;
    let instrument_id = parse_u32(parts[2])?;
    validate_instrument(instrument_id)?;
    Ok(WireCommand::CancelById {
        order_id,
        instrument_id,
    })
}

fn encode_single(cmd: &WireCommand) -> String {
    match cmd {
        WireCommand::Add {
            id,
            participant_id,
            instrument_id,
            side,
            price,
            quantity,
        } => format!(
            "ADD {id} {participant_id} {instrument_id} {} {price} {quantity}",
            encode_side(*side)
        ),
        WireCommand::Market {
            id,
            participant_id,
            instrument_id,
            side,
            quantity,
        } => format!(
            "MARKET {id} {participant_id} {instrument_id} {} {quantity}",
            encode_side(*side)
        ),
        WireCommand::CancelById {
            order_id,
            instrument_id,
        } => format!("CANCELID {order_id} {instrument_id}"),
    }
}

fn validate_instrument(instrument_id: u32) -> Result<(), WireParseError> {
    if instrument_id == 0 {
        return Err(WireParseError::InvalidInstrument);
    }
    Ok(())
}

fn parse_side(token: &str) -> Result<Side, WireParseError> {
    match token {
        "B" | "BUY" => Ok(Side::Buy),
        "S" | "SELL" => Ok(Side::Sell),
        _ => Err(WireParseError::InvalidSide),
    }
}

fn encode_side(side: Side) -> &'static str {
    match side {
        Side::Buy => "B",
        Side::Sell => "S",
    }
}

fn parse_u64(token: &str) -> Result<u64, WireParseError> {
    token
        .parse::<u64>()
        .map_err(|_| WireParseError::InvalidNumber)
}

fn parse_u32(token: &str) -> Result<u32, WireParseError> {
    token
        .parse::<u32>()
        .map_err(|_| WireParseError::InvalidNumber)
}

fn parse_i64(token: &str) -> Result<i64, WireParseError> {
    token
        .parse::<i64>()
        .map_err(|_| WireParseError::InvalidNumber)
}

#[cfg(test)]
mod tests {
    use super::{
        WireCommand, WireFrame, WireParseError, encode_frame, parse_frame,
    };
    use crate::types::Side;

    #[test]
    fn parse_single_add_roundtrip() {
        let frame = parse_frame("ADD 10 20 2 B 101 5").expect("parse");
        assert_eq!(
            frame,
            WireFrame {
                commands: vec![WireCommand::Add {
                    id: 10,
                    participant_id: 20,
                    instrument_id: 2,
                    side: Side::Buy,
                    price: 101,
                    quantity: 5
                }]
            }
        );
        let line = encode_frame(&frame).expect("encode");
        assert_eq!(line, "ADD 10 20 2 B 101 5\n");
    }

    #[test]
    fn parse_batch_frame() {
        let frame =
            parse_frame("BATCH ADD 1 2 1 B 100 1|CANCELID 1 1").expect("parse");
        assert_eq!(frame.commands.len(), 2);
    }

    #[test]
    fn reject_zero_instrument() {
        let err = parse_frame("ADD 1 2 0 B 100 1").expect_err("must fail");
        assert_eq!(err, WireParseError::InvalidInstrument);
    }
}
