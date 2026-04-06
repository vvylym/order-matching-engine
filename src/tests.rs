//! Unit tests for traits, commands, and error surfaces.

use crate::book::PriceBook;
use crate::engine::{
    AddOrderCommand, CancelByOrderIdCommand, CancelOrderCommand,
    ExecuteOrderCommand, OrderCommand, OrderMatchingService, ReduceOrderCommand,
    ReplaceOrderByNewIdCommand, ReplaceOrderCommand,
};
use crate::error::{Error, RejectionError, Result};
use crate::events::{Event, EventSinkError};
use crate::matching::{MatchingPolicy, MatchingPolicyError};
use crate::self_trade::SelfTradePolicyError;
use crate::sequence::{SequenceGenerator, SequenceGeneratorError};
use crate::store::OrderStoreError;
use crate::types::*;

// --- OrderMatchingService default `process` path ---

struct RecordingMatching {
    op: &'static str,
}

impl OrderMatchingService for RecordingMatching {
    fn add(&mut self, _cmd: AddOrderCommand) -> Result<()> {
        self.op = "add";
        Ok(())
    }

    fn cancel(&mut self, _cmd: CancelOrderCommand) -> Result<()> {
        self.op = "cancel";
        Ok(())
    }

    fn replace(&mut self, _cmd: ReplaceOrderCommand) -> Result<()> {
        self.op = "replace";
        Ok(())
    }

    fn cancel_by_order_id(&mut self, _cmd: CancelByOrderIdCommand) -> Result<()> {
        self.op = "cancel_by_order_id";
        Ok(())
    }

    fn reduce(&mut self, _cmd: ReduceOrderCommand) -> Result<()> {
        self.op = "reduce";
        Ok(())
    }

    fn execute(&mut self, _cmd: ExecuteOrderCommand) -> Result<()> {
        self.op = "execute";
        Ok(())
    }

    fn replace_by_new_id(
        &mut self,
        _cmd: ReplaceOrderByNewIdCommand,
    ) -> Result<()> {
        self.op = "replace_by_new_id";
        Ok(())
    }
}

fn sample_add_cmd() -> AddOrderCommand {
    AddOrderCommand {
        id: 1,
        participant_id: 10,
        symbol_id: 1,
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(100),
        quantity: 10,
        time_in_force: TimeInForce::Gtc,
        stop_price: None,
        max_visible_quantity: None,
        slippage: None,
        trailing_distance: None,
        trailing_step: None,
    }
}

#[test]
fn order_matching_service_process_dispatches_add_cancel_replace() {
    let mut s = RecordingMatching { op: "" };
    s.process(OrderCommand::Add(sample_add_cmd())).unwrap();
    assert_eq!(s.op, "add");

    let mut s = RecordingMatching { op: "" };
    s.process(OrderCommand::Cancel(CancelOrderCommand {
        order_id: 1,
        participant_id: 10,
        sequence: 1,
    }))
    .unwrap();
    assert_eq!(s.op, "cancel");

    let mut s = RecordingMatching { op: "" };
    s.process(OrderCommand::Replace(crate::engine::ReplaceOrderCommand {
        order_id: 1,
        participant_id: 10,
        new_price: Some(99),
        new_quantity: 5,
        sequence: 2,
    }))
    .unwrap();
    assert_eq!(s.op, "replace");
}

// --- SequenceGenerator default ---

struct CountingSeq(u64);

impl SequenceGenerator for CountingSeq {
    fn next(&mut self) -> std::result::Result<Sequence, SequenceGeneratorError> {
        self.0 = self
            .0
            .checked_add(1)
            .ok_or(SequenceGeneratorError::Overflow)?;
        Ok(self.0)
    }
}

#[test]
fn sequence_next_at_end_of_queue_defaults_to_next() {
    let mut g = CountingSeq(0);
    let a = g.next_at_end_of_queue().unwrap();
    let b = g.next_at_end_of_queue().unwrap();
    assert_eq!(a, 1);
    assert_eq!(b, 2);
}

// --- Error Display / From (thiserror) ---

#[test]
fn matching_policy_error_display() {
    let e = MatchingPolicyError::IncompatibleSides;
    assert!(e.to_string().contains("incompatible"));
    let e = MatchingPolicyError::PriceDoesNotCross {
        incoming_price: Some(10),
        resting_price: 20,
    };
    assert!(e.to_string().contains("cross"));
    assert!(
        MatchingPolicyError::UndefinedMarketPrice
            .to_string()
            .contains("market")
    );
    assert!(
        MatchingPolicyError::InvalidOrderTypeCombination
            .to_string()
            .contains("order type")
    );
    assert!(
        MatchingPolicyError::UnexpectedError("x".into())
            .to_string()
            .contains("unexpected")
    );
}

#[test]
fn sequence_generator_error_display() {
    assert!(
        SequenceGeneratorError::Overflow
            .to_string()
            .contains("overflow")
    );
    assert!(
        SequenceGeneratorError::CorruptedState("a".into())
            .to_string()
            .contains("Corrupted")
    );
    assert!(
        SequenceGeneratorError::UnexpectedError("u".into())
            .to_string()
            .contains("unexpected")
    );
}

#[test]
fn order_store_error_display() {
    assert!(
        OrderStoreError::AlreadyExists(1)
            .to_string()
            .contains("exists")
    );
    assert!(
        OrderStoreError::NotFound(2)
            .to_string()
            .contains("not found")
    );
    let e = OrderStoreError::ParticipantMismatch {
        order_id: 1,
        expected: 2,
        actual: 3,
    };
    assert!(e.to_string().contains("participant"));
    assert!(
        OrderStoreError::CorruptedState("c".into())
            .to_string()
            .contains("corrupted")
    );
    assert!(
        OrderStoreError::UnexpectedError("z".into())
            .to_string()
            .contains("unexpected")
    );
}

#[test]
fn self_trade_policy_error_display() {
    let e = SelfTradePolicyError::SameParticipant {
        participant_id: 1,
        aggressor: 2,
        resting: 3,
    };
    assert!(e.to_string().contains("participant"));
    assert!(
        SelfTradePolicyError::PolicyMisconfigured("m".into())
            .to_string()
            .contains("misconfigured")
    );
    assert!(
        SelfTradePolicyError::UnexpectedError("e".into())
            .to_string()
            .contains("unexpected")
    );
}

#[test]
fn event_sink_error_display() {
    assert!(
        EventSinkError::Disconnected
            .to_string()
            .contains("disconnected")
    );
    assert!(
        EventSinkError::Backpressure
            .to_string()
            .contains("backpressure")
    );
    assert!(
        EventSinkError::SerializationFailed("s".into())
            .to_string()
            .contains("serialization")
    );
    assert!(
        EventSinkError::UnexpectedError("t".into())
            .to_string()
            .contains("unexpected")
    );
}

#[test]
fn rejection_error_display_samples() {
    assert!(RejectionError::InvalidPrice.to_string().contains("price"));
    assert!(RejectionError::OrderNotFound(9).to_string().contains("9"));
    assert!(
        RejectionError::SymbolDuplicate
            .to_string()
            .contains("symbol")
    );
}

#[test]
fn top_level_error_from_sources() {
    let e: Error = MatchingPolicyError::IncompatibleSides.into();
    assert!(e.to_string().contains("matching"));
    let e: Error = SequenceGeneratorError::Overflow.into();
    assert!(e.to_string().contains("sequence"));
    let e: Error = OrderStoreError::NotFound(1).into();
    assert!(e.to_string().contains("order store"));
    let e: Error = SelfTradePolicyError::UnexpectedError("".into()).into();
    assert!(e.to_string().contains("self trade"));
    let e: Error = EventSinkError::Backpressure.into();
    assert!(e.to_string().contains("event"));
    let e: Error = RejectionError::StaleSequence.into();
    assert!(e.to_string().contains("rejection"));
}

// --- Types ---

#[test]
fn types_symbol_roundtrip_partial_eq() {
    let s = Symbol {
        id: 1,
        name: *b"TEST    ",
    };
    assert_eq!(s, s);
    assert_eq!(Side::Buy, Side::Buy);
    assert_ne!(Side::Buy, Side::Sell);
}

#[test]
fn event_variants_clone() {
    let t = Trade {
        aggressor: 1,
        resting: 2,
        price: 10,
        quantity: 1,
        sequence: 1,
    };
    let e = Event::Trade(t.clone());
    assert!(matches!(e.clone(), Event::Trade(_)));
    assert!(matches!(
        Event::Rejected(RejectionError::SelfTrade),
        Event::Rejected(_)
    ));
}

// --- Mock-generated traits still compile: smoke call ---

#[test]
fn mockall_traits_instantiate() {
    let mut book = crate::book::MockPriceBook::new();
    book.expect_best_bid().return_const(Some(1i64));
    assert_eq!(book.best_bid(), Some(1));

    let mut pol = crate::matching::MockMatchingPolicy::new();
    pol.expect_can_match().returning(|_, _| Ok(true));
    let o = Order {
        symbol_id: 1,
        id: 1,
        participant_id: 1,
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(10),
        quantity: 1,
        time_in_force: TimeInForce::Gtc,
        stop_price: None,
        max_visible_quantity: None,
        slippage: None,
        trailing_distance: None,
        trailing_step: None,
        executed_quantity: 0,
        leaves_quantity: 1,
        sequence: 1,
    };
    assert!(pol.can_match(&o, &o).unwrap());
}
