//! Experimental rkyv binary encoding for harness [`super::WireFrame`] values.
//!
//! This is **not** wired into the TCP harness yet; it provides a canonical
//! serialized layout for future binary transports and microbenchmarks.

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use super::{MAX_BATCH_COMMANDS, WireCommand, WireFrame, WireParseError};

/// Failure to encode or decode an rkyv wire payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RkyvWireError {
    /// Frame-level validation failed before serialization.
    Parse(WireParseError),
    /// rkyv serialization or deserialization failed.
    Rkyv(&'static str),
}

impl std::fmt::Display for RkyvWireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "{e}"),
            Self::Rkyv(msg) => write!(f, "rkyv: {msg}"),
        }
    }
}

impl std::error::Error for RkyvWireError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            Self::Rkyv(_) => None,
        }
    }
}

impl From<WireParseError> for RkyvWireError {
    fn from(value: WireParseError) -> Self {
        Self::Parse(value)
    }
}

type RkyvEncodeResult = Result<Vec<u8>, RkyvWireError>;

type RkyvDecodeResult = Result<WireFrame, RkyvWireError>;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub(crate) enum RkyvSide {
    Buy,
    Sell,
}

impl From<crate::types::Side> for RkyvSide {
    fn from(value: crate::types::Side) -> Self {
        match value {
            crate::types::Side::Buy => Self::Buy,
            crate::types::Side::Sell => Self::Sell,
        }
    }
}

impl From<RkyvSide> for crate::types::Side {
    fn from(value: RkyvSide) -> Self {
        match value {
            RkyvSide::Buy => Self::Buy,
            RkyvSide::Sell => Self::Sell,
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub(crate) enum RkyvWireCommand {
    Add {
        id: u64,
        participant_id: u64,
        instrument_id: u32,
        side: RkyvSide,
        price: i64,
        quantity: i64,
    },
    Market {
        id: u64,
        participant_id: u64,
        instrument_id: u32,
        side: RkyvSide,
        quantity: i64,
    },
    CancelById {
        order_id: u64,
        instrument_id: u32,
    },
}

impl From<&WireCommand> for RkyvWireCommand {
    fn from(value: &WireCommand) -> Self {
        match value {
            WireCommand::Add {
                id,
                participant_id,
                instrument_id,
                side,
                price,
                quantity,
            } => Self::Add {
                id: *id,
                participant_id: *participant_id,
                instrument_id: *instrument_id,
                side: (*side).into(),
                price: *price,
                quantity: *quantity,
            },
            WireCommand::Market {
                id,
                participant_id,
                instrument_id,
                side,
                quantity,
            } => Self::Market {
                id: *id,
                participant_id: *participant_id,
                instrument_id: *instrument_id,
                side: (*side).into(),
                quantity: *quantity,
            },
            WireCommand::CancelById {
                order_id,
                instrument_id,
            } => Self::CancelById {
                order_id: *order_id,
                instrument_id: *instrument_id,
            },
        }
    }
}

impl From<RkyvWireCommand> for WireCommand {
    fn from(value: RkyvWireCommand) -> Self {
        match value {
            RkyvWireCommand::Add {
                id,
                participant_id,
                instrument_id,
                side,
                price,
                quantity,
            } => Self::Add {
                id,
                participant_id,
                instrument_id,
                side: side.into(),
                price,
                quantity,
            },
            RkyvWireCommand::Market {
                id,
                participant_id,
                instrument_id,
                side,
                quantity,
            } => Self::Market {
                id,
                participant_id,
                instrument_id,
                side: side.into(),
                quantity,
            },
            RkyvWireCommand::CancelById {
                order_id,
                instrument_id,
            } => Self::CancelById {
                order_id,
                instrument_id,
            },
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub(crate) struct RkyvWireFrame {
    pub commands: Vec<RkyvWireCommand>,
}

fn validated_rkyv_frame(
    frame: &WireFrame,
) -> Result<RkyvWireFrame, WireParseError> {
    if frame.commands.is_empty() {
        return Err(WireParseError::InvalidBatch);
    }
    if frame.commands.len() > MAX_BATCH_COMMANDS {
        return Err(WireParseError::BatchTooLarge);
    }
    let commands = frame.commands.iter().map(RkyvWireCommand::from).collect();
    Ok(RkyvWireFrame { commands })
}

fn rkyv_frame_to_wire(frame: RkyvWireFrame) -> Result<WireFrame, WireParseError> {
    if frame.commands.is_empty() {
        return Err(WireParseError::InvalidBatch);
    }
    if frame.commands.len() > MAX_BATCH_COMMANDS {
        return Err(WireParseError::BatchTooLarge);
    }
    let mut commands = smallvec::SmallVec::with_capacity(frame.commands.len());
    for cmd in frame.commands {
        commands.push(WireCommand::from(cmd));
    }
    Ok(WireFrame { commands })
}

/// Serialize a wire frame to rkyv bytes (owned vector, aligned for access).
pub fn encode_frame_rkyv(frame: &WireFrame) -> RkyvEncodeResult {
    let payload = validated_rkyv_frame(frame)?;
    let aligned = rkyv::to_bytes::<rkyv::rancor::Error>(&payload)
        .map_err(|_| RkyvWireError::Rkyv("serialize"))?;
    Ok(aligned.into_vec())
}

/// Deserialize rkyv bytes into a wire frame.
pub fn decode_frame_rkyv(bytes: &[u8]) -> RkyvDecodeResult {
    let frame: RkyvWireFrame =
        rkyv::from_bytes::<RkyvWireFrame, rkyv::rancor::Error>(bytes)
            .map_err(|_| RkyvWireError::Rkyv("deserialize"))?;
    rkyv_frame_to_wire(frame).map_err(RkyvWireError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Side;
    use smallvec::smallvec;

    #[test]
    fn rkyv_roundtrip_batch() {
        let frame = WireFrame {
            commands: smallvec![
                WireCommand::Add {
                    id: 1,
                    participant_id: 2,
                    instrument_id: 3,
                    side: Side::Buy,
                    price: 100,
                    quantity: 5,
                },
                WireCommand::CancelById {
                    order_id: 1,
                    instrument_id: 3,
                },
            ],
        };
        let bytes = encode_frame_rkyv(&frame).expect("encode");
        let decoded = decode_frame_rkyv(&bytes).expect("decode");
        assert_eq!(decoded, frame);
    }
}
