//! Order matching engine implementation.
//!
//! Single-writer authority: `OrderStore` is truth, `PriceBook` is derived, `EventSink` is append-only.
//! Validation and matching steps are grouped with clear section comments.
//!
use super::*;
use crate::book::PriceBook;
use crate::events::Event;
use crate::events::EventSink;
use crate::matching::MatchingPolicy;
use crate::self_trade::SelfTradePolicy;
use crate::sequence::SequenceGenerator;
use crate::store::OrderStore;

use crate::error::*;
use crate::store::OrderStoreError;
use crate::types::*;

/// Order matching engine
///
/// Applies add/cancel/replace commands against store, book, and event sink.
/// Store is authoritative; book is derived from store; events are append-only
/// for replay and audit.
#[derive(Debug, Clone)]
pub struct OrderMatchingEngine<SG, PB, OS, MP, STP, ES>
where
    SG: SequenceGenerator,
    PB: PriceBook,
    OS: OrderStore,
    MP: MatchingPolicy,
    STP: SelfTradePolicy,
    ES: EventSink,
{
    /// Sequence generator
    sequence_generator: SG,
    /// Price book
    price_book: PB,
    /// Order store
    order_store: OS,
    /// Matching policy
    matching_policy: MP,
    /// Self trade policy
    self_trade_policy: STP,
    /// Event sink
    event_sink: ES,
}

impl<SG, PB, OS, MP, STP, ES> OrderMatchingEngine<SG, PB, OS, MP, STP, ES>
where
    SG: SequenceGenerator,
    PB: PriceBook,
    OS: OrderStore,
    MP: MatchingPolicy,
    STP: SelfTradePolicy,
    ES: EventSink,
{
    /// Builds a new engine with the given sequence generator, price book, order store, and policies.
    pub fn new(
        sequence_generator: SG,
        price_book: PB,
        order_store: OS,
        matching_policy: MP,
        self_trade_policy: STP,
        event_sink: ES,
    ) -> Self {
        Self {
            sequence_generator,
            price_book,
            order_store,
            matching_policy,
            self_trade_policy,
            event_sink,
        }
    }

    /// Best bid price (for tests and introspection).
    pub fn best_bid(&self) -> Option<crate::types::Price> {
        self.price_book.best_bid()
    }

    /// Best ask price (for tests and introspection).
    pub fn best_ask(&self) -> Option<crate::types::Price> {
        self.price_book.best_ask()
    }

    /// Emits a rejection event then returns `Err`, so callers need not emit separately.
    fn emit_rejection(&self, rejection_error: RejectionError) -> Result<()> {
        self.event_sink.emit(Event::Rejected(rejection_error))?;
        Err(Error::from(rejection_error))
    }

    /// If `r` is `Err(e)`, emits rejection and returns; otherwise continues. Use with module validators.
    fn or_emit_rejection(
        &self,
        r: std::result::Result<(), RejectionError>,
    ) -> Result<()> {
        match r {
            Ok(()) => Ok(()),
            Err(e) => self.emit_rejection(e),
        }
    }

    /// Looks up order by id; if found runs `f(order)`, otherwise emits OrderNotFound.
    fn with_order_validate<F>(&self, order_id: OrderId, f: F) -> Result<()>
    where
        F: FnOnce(&Order) -> Result<()>,
    {
        match self.order_store.get(&order_id) {
            Some(order) => f(&order),
            None => self.emit_rejection(RejectionError::OrderNotFound(order_id)),
        }
    }

    /// Runs the matching loop then commits any remainder (IOC ⇒ reject unfilled; GTC ⇒ rest in book).
    fn match_and_commit_incoming(&mut self, incoming: &mut Order) -> Result<()> {
        // ─── Matching loop: consume liquidity until none left or order no longer marketable ───
        while incoming.quantity > 0 {
            let resting_id = match self.price_book.pop_best(incoming.side) {
                Some(id) => id,
                None => break,
            };
            let mut resting = self.order_store.remove(&resting_id)?;
            let resting_price = match resting.price {
                Some(p) => p,
                None => {
                    self.order_store.insert(&resting)?;
                    return self.emit_rejection(
                        RejectionError::PriceBookInvariantViolation,
                    );
                }
            };

            if self.self_trade_policy.violates(incoming, &resting)? {
                self.order_store.insert(&resting)?;
                self.price_book.push(
                    &resting_price,
                    resting.id,
                    resting.side,
                    resting.sequence,
                );
                return self.emit_rejection(RejectionError::SelfTrade);
            }

            if !self.matching_policy.can_match(incoming, &resting)? {
                self.order_store.insert(&resting)?;
                self.price_book.push(
                    &resting_price,
                    resting.id,
                    resting.side,
                    resting.sequence,
                );
                break;
            }

            let traded_qty = incoming.quantity.min(resting.leaves_quantity);
            incoming.quantity -= traded_qty;
            incoming.leaves_quantity = incoming.quantity;
            let remaining_qty = resting.leaves_quantity - traded_qty;

            if remaining_qty > 0 {
                resting.executed_quantity += traded_qty;
                resting.leaves_quantity = remaining_qty;
                self.order_store.insert(&resting)?;
                self.price_book.push(
                    &resting_price,
                    resting.id,
                    resting.side,
                    resting.sequence,
                );
            }

            self.event_sink.emit(Event::Trade(Trade {
                aggressor: incoming.id,
                resting: resting.id,
                price: resting_price,
                quantity: traded_qty,
                sequence: self.sequence_generator.next()?,
            }))?;
        }

        // ─── Commit remainder: IOC rejects unfilled; GTC rests in store and book, then Accepted ───
        if incoming.quantity > 0 {
            match incoming.time_in_force {
                TimeInForce::Ioc | TimeInForce::Fok | TimeInForce::Aon => {
                    return self
                        .emit_rejection(RejectionError::InsufficientLiquidity);
                }
                TimeInForce::Gtc => {
                    self.order_store.insert(incoming)?;
                    if let Some(price) = incoming.price {
                        self.price_book.push(
                            &price,
                            incoming.id,
                            incoming.side,
                            incoming.sequence,
                        );
                    }
                    self.event_sink.emit(Event::Accepted(incoming.id))?;
                }
            }
        }

        Ok(())
    }

    /// Applies [`OrderCommand`]s in order via [`OrderMatchingService::process`].
    ///
    /// On the first error, processing stops and the error is returned; prior commands remain
    /// committed. Intended for feed replay, gateways, and throughput experiments (see
    /// **`throughput_engine`** bench).
    pub fn process_batch(
        &mut self,
        cmds: impl IntoIterator<Item = OrderCommand>,
    ) -> Result<()> {
        for cmd in cmds {
            OrderMatchingService::process(self, cmd)?;
        }
        Ok(())
    }
}

impl<SG, PB, OS, MP, STP, ES> OrderMatchingService
    for OrderMatchingEngine<SG, PB, OS, MP, STP, ES>
where
    SG: SequenceGenerator,
    PB: PriceBook,
    OS: OrderStore,
    MP: MatchingPolicy,
    STP: SelfTradePolicy,
    ES: EventSink,
{
    fn add(&mut self, cmd: AddOrderCommand) -> Result<()> {
        // ─── ADD: Validation ───
        // 1. Reject non-positive quantity.
        if cmd.quantity <= 0 {
            return self.emit_rejection(RejectionError::InvalidQuantity);
        }
        // 2. Reject invalid price for order type (Limit ⇒ Some(price), Market ⇒ None).
        self.or_emit_rejection(validate_order_price(cmd.order_type, cmd.price))?;

        // ─── ADD: Build order and match ───
        // 3. Assign sequence and build Order from command.
        let sequence = self.sequence_generator.next()?;

        let mut incoming = Order {
            symbol_id: cmd.symbol_id,
            id: cmd.id,
            participant_id: cmd.participant_id,
            side: cmd.side,
            order_type: cmd.order_type,
            price: cmd.price,
            quantity: cmd.quantity,
            time_in_force: cmd.time_in_force,
            stop_price: cmd.stop_price,
            max_visible_quantity: cmd.max_visible_quantity,
            slippage: cmd.slippage,
            trailing_distance: cmd.trailing_distance,
            trailing_step: cmd.trailing_step,
            executed_quantity: 0,
            leaves_quantity: cmd.quantity,
            sequence,
        };

        // 4. Run matching loop and commit remainder (IOC/GTC).
        self.match_and_commit_incoming(&mut incoming)
    }

    // ─── OrderMatchingService: cancel ───
    fn cancel(&mut self, cmd: CancelOrderCommand) -> Result<()> {
        // ─── CANCEL: Validation ───
        // 1. Order must exist; participant and sequence must match (ownership).
        self.with_order_validate(cmd.order_id, |order| {
            self.or_emit_rejection(validate_order_ownership(
                order,
                cmd.participant_id,
                cmd.sequence,
            ))
        })?;

        // ─── CANCEL: Remove from store and book ───
        // 2. Remove from store (authoritative); then from book (derived).
        let order = self.order_store.remove(&cmd.order_id)?;
        if !self.price_book.remove(&order.id) {
            return self
                .emit_rejection(RejectionError::PriceBookInvariantViolation);
        }

        // 3. Emit Canceled for downstream and audit.
        self.event_sink.emit(Event::Canceled(order.id))?;
        Ok(())
    }

    fn replace(&mut self, cmd: ReplaceOrderCommand) -> Result<()> {
        // ─── REPLACE: Validation (no state change until all checks pass) ───
        // 1. Reject non-positive new quantity.
        if cmd.new_quantity <= 0 {
            return self.emit_rejection(RejectionError::InvalidQuantity);
        }
        // 2. Order must exist; participant and sequence must match; new price must fit order type.
        self.with_order_validate(cmd.order_id, |order| {
            self.or_emit_rejection(validate_order_ownership(
                order,
                cmd.participant_id,
                cmd.sequence,
            ))?;
            self.or_emit_rejection(validate_order_price(
                order.order_type,
                cmd.new_price,
            ))
        })?;

        // ─── REPLACE: Cancel old order (store and book) ───
        // 3. Remove old from store then book; emit Canceled (replace = cancel + add semantics).
        let old = self.order_store.remove(&cmd.order_id)?;
        if !self.price_book.remove(&old.id) {
            return self
                .emit_rejection(RejectionError::PriceBookInvariantViolation);
        }
        self.event_sink.emit(Event::Canceled(old.id))?;

        // ─── REPLACE: Build new order and match ───
        // 4. Assign sequence (e.g. end-of-queue for replaced orders); build Order with same id.
        let sequence = self.sequence_generator.next_at_end_of_queue()?;

        let mut new = Order {
            symbol_id: old.symbol_id,
            id: old.id,
            participant_id: old.participant_id,
            side: old.side,
            order_type: old.order_type,
            price: cmd.new_price,
            quantity: cmd.new_quantity,
            time_in_force: old.time_in_force,
            executed_quantity: 0,
            leaves_quantity: cmd.new_quantity,
            sequence,
            ..old
        };
        new.price = cmd.new_price;
        new.quantity = cmd.new_quantity;
        new.leaves_quantity = cmd.new_quantity;

        // 5. Run matching loop and commit remainder (same as add; IOC/GTC).
        self.match_and_commit_incoming(&mut new)
    }

    fn cancel_by_order_id(&mut self, cmd: CancelByOrderIdCommand) -> Result<()> {
        let order = match self.order_store.remove(&cmd.order_id) {
            Ok(o) => o,
            Err(OrderStoreError::NotFound(_)) => {
                return self
                    .emit_rejection(RejectionError::OrderNotFound(cmd.order_id));
            }
            Err(e) => return Err(e.into()),
        };
        if !self.price_book.remove(&order.id) {
            return self
                .emit_rejection(RejectionError::PriceBookInvariantViolation);
        }
        self.event_sink.emit(Event::Canceled(order.id))?;
        Ok(())
    }

    fn reduce(&mut self, cmd: ReduceOrderCommand) -> Result<()> {
        if cmd.quantity <= 0 {
            return self.emit_rejection(RejectionError::InvalidQuantity);
        }
        let mut order = match self.order_store.get(&cmd.order_id) {
            Some(o) => o.clone(),
            None => {
                return self
                    .emit_rejection(RejectionError::OrderNotFound(cmd.order_id));
            }
        };
        if cmd.quantity >= order.leaves_quantity {
            return self.process(OrderCommand::CancelByOrderId(
                CancelByOrderIdCommand {
                    order_id: cmd.order_id,
                },
            ));
        }
        order.leaves_quantity -= cmd.quantity;
        let price = match order.price {
            Some(p) => p,
            None => {
                return self
                    .emit_rejection(RejectionError::OrderParameterInvalid);
            }
        };
        self.order_store.remove(&order.id)?;
        self.price_book.remove(&order.id);
        self.order_store.insert(&order)?;
        self.price_book
            .push(&price, order.id, order.side, order.sequence);
        Ok(())
    }

    fn execute(&mut self, cmd: ExecuteOrderCommand) -> Result<()> {
        if cmd.quantity <= 0 {
            return self.emit_rejection(RejectionError::InvalidQuantity);
        }
        let mut order = match self.order_store.get(&cmd.order_id) {
            Some(o) => o.clone(),
            None => {
                return self
                    .emit_rejection(RejectionError::OrderNotFound(cmd.order_id));
            }
        };
        order.executed_quantity += cmd.quantity;
        order.leaves_quantity -= cmd.quantity;
        if order.leaves_quantity <= 0 {
            self.order_store.remove(&order.id)?;
            self.price_book.remove(&order.id);
        } else {
            let price = match order.price {
                Some(p) => p,
                None => {
                    return self
                        .emit_rejection(RejectionError::OrderParameterInvalid);
                }
            };
            self.order_store.remove(&order.id)?;
            self.price_book.remove(&order.id);
            self.order_store.insert(&order)?;
            self.price_book
                .push(&price, order.id, order.side, order.sequence);
        }
        Ok(())
    }

    fn replace_by_new_id(
        &mut self,
        cmd: ReplaceOrderByNewIdCommand,
    ) -> Result<()> {
        if cmd.new_quantity <= 0 {
            return self.emit_rejection(RejectionError::InvalidQuantity);
        }
        let old = match self.order_store.remove(&cmd.old_order_id) {
            Ok(o) => o,
            Err(OrderStoreError::NotFound(_)) => {
                return self.emit_rejection(RejectionError::OrderNotFound(
                    cmd.old_order_id,
                ));
            }
            Err(e) => return Err(e.into()),
        };
        if !self.price_book.remove(&old.id) {
            return self
                .emit_rejection(RejectionError::PriceBookInvariantViolation);
        }
        self.event_sink.emit(Event::Canceled(old.id))?;

        let sequence = self.sequence_generator.next_at_end_of_queue()?;
        let symbol_id = cmd.symbol_id.unwrap_or(old.symbol_id);
        let side = cmd.side.unwrap_or(old.side);
        let mut new = Order {
            symbol_id,
            id: cmd.new_order_id,
            participant_id: old.participant_id,
            side,
            order_type: OrderType::Limit,
            price: Some(cmd.new_price),
            quantity: cmd.new_quantity,
            time_in_force: TimeInForce::Gtc,
            executed_quantity: 0,
            leaves_quantity: cmd.new_quantity,
            sequence,
            ..Order::default()
        };
        self.match_and_commit_incoming(&mut new)
    }
}

/// Returns `Ok(())` iff price is consistent with order type (Limit ⇒ Some(price), Market ⇒ None).
fn validate_order_price(
    order_type: OrderType,
    price: Option<Price>,
) -> std::result::Result<(), RejectionError> {
    match (order_type, price) {
        (OrderType::Limit, Some(_)) | (OrderType::Market, None) => Ok(()),
        _ => Err(RejectionError::InvalidPrice),
    }
}

/// Returns `Ok(())` iff participant and sequence match the stored order (for cancel/replace).
fn validate_order_ownership(
    order: &Order,
    participant_id: ParticipantId,
    sequence: Sequence,
) -> std::result::Result<(), RejectionError> {
    if participant_id != order.participant_id {
        return Err(RejectionError::ParticipantMismatch);
    }
    if sequence != order.sequence {
        return Err(RejectionError::StaleSequence);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::MockPriceBook;
    use crate::events::{EventSinkError, MockEventSink};
    use crate::matching::{MatchingPolicyError, MockMatchingPolicy};
    use crate::self_trade::{MockSelfTradePolicy, SelfTradePolicyError};
    use crate::sequence::{MockSequenceGenerator, SequenceGeneratorError};
    use crate::store::{MockOrderStore, OrderStoreError};

    type MockEngine = OrderMatchingEngine<
        MockSequenceGenerator,
        MockPriceBook,
        MockOrderStore,
        MockMatchingPolicy,
        MockSelfTradePolicy,
        MockEventSink,
    >;

    fn mock_engine(
        seq: MockSequenceGenerator,
        book: MockPriceBook,
        store: MockOrderStore,
        matching: MockMatchingPolicy,
        self_trade: MockSelfTradePolicy,
        sink: MockEventSink,
    ) -> MockEngine {
        OrderMatchingEngine::new(seq, book, store, matching, self_trade, sink)
    }

    /// Cover emit_rejection when event_sink.emit returns Err (line 70 ?).
    #[test]
    /// Cover emit_rejection path when event_sink.emit(Rejected) fails.
    fn emit_rejection_when_sink_fails() {
        let seq = MockSequenceGenerator::new();
        let book = MockPriceBook::new();
        let store = MockOrderStore::new();
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let mut sink = MockEventSink::new();
        sink.expect_emit()
            .return_once(|_| Err(EventSinkError::Disconnected));
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = AddOrderCommand {
            id: 1,
            participant_id: 100,
            symbol_id: 0,
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(50),
            quantity: 0, // triggers InvalidQuantity -> emit_rejection
            time_in_force: TimeInForce::Gtc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
        };
        let res = engine.add(cmd);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::EventSink(_)));
    }

    /// Cover sequence_generator.next()? in add (line 99).
    #[test]
    fn add_sequence_error() {
        let mut seq = MockSequenceGenerator::new();
        seq.expect_next()
            .return_once(|| Err(SequenceGeneratorError::Overflow));
        let book = MockPriceBook::new();
        let store = MockOrderStore::new();
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let sink = MockEventSink::new();
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = AddOrderCommand {
            id: 1,
            participant_id: 100,
            symbol_id: 0,
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(50),
            quantity: 10,
            time_in_force: TimeInForce::Gtc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
        };
        let res = engine.add(cmd);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::Sequence(_)));
    }

    /// Cover add: Market order with price present -> InvalidPrice (explicit guard).
    #[test]
    fn add_market_order_with_price_rejected() {
        let seq = MockSequenceGenerator::new();
        let book = MockPriceBook::new();
        let store = MockOrderStore::new();
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let mut sink = MockEventSink::new();
        sink.expect_emit().returning(|_| Ok(()));
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = AddOrderCommand {
            id: 1,
            participant_id: 100,
            symbol_id: 0,
            side: Side::Buy,
            order_type: OrderType::Market,
            price: Some(50), // invalid: market must have no price
            quantity: 10,
            time_in_force: TimeInForce::Gtc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
        };
        let res = engine.add(cmd);
        assert!(res.is_err());
        assert!(matches!(
            res.unwrap_err(),
            Error::Rejection(RejectionError::InvalidPrice)
        ));
    }

    /// Cover resting.price None -> PriceBookInvariantViolation (lines 122-124).
    #[test]
    fn add_resting_order_with_none_price_invariant_violation() {
        let mut seq = MockSequenceGenerator::new();
        seq.expect_next().return_once(|| Ok(0));
        let mut book = MockPriceBook::new();
        let order_no_price = Order {
            id: 1,
            participant_id: 100,
            side: Side::Sell,
            order_type: OrderType::Limit,
            price: None, // invalid for resting limit
            quantity: 10,
            time_in_force: TimeInForce::Gtc,
            sequence: 0,
            leaves_quantity: 10,
            executed_quantity: 0,
            ..Order::default()
        };
        book.expect_pop_best().return_once(|_| Some(1));
        let mut store = MockOrderStore::new();
        store
            .expect_remove()
            .return_once(move |_| Ok(order_no_price));
        store.expect_insert().return_once(|_| Ok(()));
        let matching = MockMatchingPolicy::new();
        let mut self_trade = MockSelfTradePolicy::new();
        self_trade.expect_violates().returning(|_, _| Ok(false));
        let mut sink = MockEventSink::new();
        sink.expect_emit().returning(|_| Ok(()));
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = AddOrderCommand {
            id: 2,
            participant_id: 101,
            symbol_id: 0,
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(50),
            quantity: 10,
            time_in_force: TimeInForce::Gtc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
        };
        let res = engine.add(cmd);
        assert!(res.is_err());
        assert!(matches!(
            res.unwrap_err(),
            Error::Rejection(RejectionError::PriceBookInvariantViolation)
        ));
    }

    /// Cover self_trade_policy.violates? returning Err (line 128).
    #[test]
    fn add_self_trade_policy_returns_err() {
        let mut seq = MockSequenceGenerator::new();
        seq.expect_next().return_once(|| Ok(0));
        let mut book = MockPriceBook::new();
        let resting = Order {
            id: 1,
            participant_id: 100,
            side: Side::Sell,
            order_type: OrderType::Limit,
            price: Some(50),
            quantity: 10,
            time_in_force: TimeInForce::Gtc,
            sequence: 0,
            leaves_quantity: 10,
            executed_quantity: 0,
            ..Order::default()
        };
        book.expect_pop_best().return_once(|_| Some(1));
        let mut store = MockOrderStore::new();
        store.expect_remove().return_once(move |_| Ok(resting));
        let matching = MockMatchingPolicy::new();
        let mut self_trade = MockSelfTradePolicy::new();
        self_trade.expect_violates().return_once(|_, _| {
            Err(SelfTradePolicyError::PolicyMisconfigured("x".into()))
        });
        let sink = MockEventSink::new();
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = AddOrderCommand {
            id: 2,
            participant_id: 101,
            symbol_id: 0,
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(50),
            quantity: 10,
            time_in_force: TimeInForce::Gtc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
        };
        let res = engine.add(cmd);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::SelfTradePolicy(_)));
    }

    /// Cover matching_policy.can_match? returning Err (line 132).
    #[test]
    fn add_matching_policy_returns_err() {
        let mut seq = MockSequenceGenerator::new();
        seq.expect_next().return_once(|| Ok(0));
        let mut book = MockPriceBook::new();
        let resting = Order {
            id: 1,
            participant_id: 100,
            side: Side::Sell,
            order_type: OrderType::Limit,
            price: Some(50),
            quantity: 10,
            time_in_force: TimeInForce::Gtc,
            sequence: 0,
            leaves_quantity: 10,
            executed_quantity: 0,
            ..Order::default()
        };
        book.expect_pop_best().return_once(|_| Some(1));
        let mut store = MockOrderStore::new();
        store.expect_remove().return_once(move |_| Ok(resting));
        let mut matching = MockMatchingPolicy::new();
        matching
            .expect_can_match()
            .return_once(|_, _| Err(MatchingPolicyError::IncompatibleSides));
        let mut self_trade = MockSelfTradePolicy::new();
        self_trade.expect_violates().returning(|_, _| Ok(false));
        let sink = MockEventSink::new();
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = AddOrderCommand {
            id: 2,
            participant_id: 101,
            symbol_id: 0,
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(50),
            quantity: 10,
            time_in_force: TimeInForce::Gtc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
        };
        let res = engine.add(cmd);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::MatchingPolicy(_)));
    }

    /// Cover order_store.remove? in add (line 142).
    #[test]
    fn add_order_store_remove_err() {
        let mut seq = MockSequenceGenerator::new();
        seq.expect_next().returning(|| Ok(0));
        let mut book = MockPriceBook::new();
        book.expect_pop_best().return_once(|_| Some(1));
        let mut store = MockOrderStore::new();
        store
            .expect_remove()
            .return_once(|_| Err(OrderStoreError::NotFound(1)));
        let mut matching = MockMatchingPolicy::new();
        matching.expect_can_match().returning(|_, _| Ok(true));
        let mut self_trade = MockSelfTradePolicy::new();
        self_trade.expect_violates().returning(|_, _| Ok(false));
        let sink = MockEventSink::new();
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = AddOrderCommand {
            id: 2,
            participant_id: 101,
            symbol_id: 0,
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(50),
            quantity: 5,
            time_in_force: TimeInForce::Gtc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
        };
        let res = engine.add(cmd);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::OrderStore(_)));
    }

    /// Cover order_store.insert(updated)? in add (line 149) and sequence.next()? in Trade (line 158).
    #[test]
    fn add_partial_fill_store_insert_err() {
        let mut seq = MockSequenceGenerator::new();
        seq.expect_next().returning(|| Ok(0));
        let mut book = MockPriceBook::new();
        let resting = Order {
            id: 1,
            participant_id: 100,
            side: Side::Sell,
            order_type: OrderType::Limit,
            price: Some(50),
            quantity: 10,
            time_in_force: TimeInForce::Gtc,
            sequence: 0,
            leaves_quantity: 10,
            executed_quantity: 0,
            ..Order::default()
        };
        let pop_count = std::cell::RefCell::new(0u32);
        let resting_clone = resting.clone();
        book.expect_pop_best().returning(move |_| {
            let mut c = pop_count.borrow_mut();
            *c += 1;
            if *c == 1 {
                Some(resting_clone.id)
            } else {
                None
            }
        });
        let mut store = MockOrderStore::new();
        store.expect_remove().returning(|_| {
            Ok(Order {
                id: 1,
                participant_id: 100,
                side: Side::Sell,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store
            .expect_insert()
            .return_once(|_| Err(OrderStoreError::AlreadyExists(1)));
        let mut matching = MockMatchingPolicy::new();
        matching.expect_can_match().returning(|_, _| Ok(true));
        let mut self_trade = MockSelfTradePolicy::new();
        self_trade.expect_violates().returning(|_, _| Ok(false));
        let mut sink = MockEventSink::new();
        sink.expect_emit().returning(|_| Ok(()));
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = AddOrderCommand {
            id: 2,
            participant_id: 101,
            symbol_id: 0,
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(50),
            quantity: 15,
            time_in_force: TimeInForce::Gtc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
        };
        let res = engine.add(cmd);
        assert!(res.is_err());
    }

    /// Cover event_sink.emit(Event::Accepted)? in add (line 174).
    #[test]
    fn add_gtc_remainder_emit_accepted_err() {
        let mut seq = MockSequenceGenerator::new();
        seq.expect_next().return_once(|| Ok(0));
        let mut book = MockPriceBook::new();
        book.expect_pop_best().returning(|_| None);
        book.expect_push().returning(|_, _, _, _| {});
        let mut store = MockOrderStore::new();
        store.expect_insert().returning(|_| Ok(()));
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let mut sink = MockEventSink::new();
        sink.expect_emit().returning(|e| match e {
            Event::Accepted(_) => Err(EventSinkError::Disconnected),
            _ => Ok(()),
        });
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = AddOrderCommand {
            id: 1,
            participant_id: 100,
            symbol_id: 0,
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(50),
            quantity: 10,
            time_in_force: TimeInForce::Gtc,
            stop_price: None,
            max_visible_quantity: None,
            slippage: None,
            trailing_distance: None,
            trailing_step: None,
        };
        let res = engine.add(cmd);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::EventSink(_)));
    }

    /// Cover cancel: order_store.remove? (line 195) and event_sink.emit(Canceled)? (line 201).
    #[test]
    fn cancel_store_remove_err() {
        let seq = MockSequenceGenerator::new();
        let book = MockPriceBook::new();
        let mut store = MockOrderStore::new();
        store.expect_get().return_once(|&id| {
            Some(Order {
                id,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store
            .expect_remove()
            .return_once(|_| Err(OrderStoreError::NotFound(1)));
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let sink = MockEventSink::new();
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = CancelOrderCommand {
            order_id: 1,
            participant_id: 100,
            sequence: 0,
        };
        let res = engine.cancel(cmd);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::OrderStore(_)));
    }

    #[test]
    fn cancel_emit_canceled_err() {
        let seq = MockSequenceGenerator::new();
        let mut book = MockPriceBook::new();
        book.expect_remove().return_once(|_| true);
        let mut store = MockOrderStore::new();
        store.expect_get().return_once(|&id| {
            Some(Order {
                id,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store.expect_remove().return_once(|_| {
            Ok(Order {
                id: 1,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let mut sink = MockEventSink::new();
        sink.expect_emit().return_once(|e| match e {
            Event::Canceled(_) => Err(EventSinkError::Disconnected),
            _ => Ok(()),
        });
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = CancelOrderCommand {
            order_id: 1,
            participant_id: 100,
            sequence: 0,
        };
        let res = engine.cancel(cmd);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::EventSink(_)));
    }

    /// Cover cancel: price_book.remove returns None -> PriceBookInvariantViolation (lines 196-199).
    #[test]
    fn cancel_book_remove_none_invariant_violation() {
        let seq = MockSequenceGenerator::new();
        let mut book = MockPriceBook::new();
        book.expect_remove().return_once(|_| false);
        let mut store = MockOrderStore::new();
        store.expect_get().return_once(|&id| {
            Some(Order {
                id,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store.expect_remove().return_once(|_| {
            Ok(Order {
                id: 1,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let mut sink = MockEventSink::new();
        sink.expect_emit().returning(|_| Ok(()));
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = CancelOrderCommand {
            order_id: 1,
            participant_id: 100,
            sequence: 0,
        };
        let res = engine.cancel(cmd);
        assert!(res.is_err());
        assert!(matches!(
            res.unwrap_err(),
            Error::Rejection(RejectionError::PriceBookInvariantViolation)
        ));
    }

    /// Cover replace: order_store.remove? (line 223), emit Canceled? (236), sequence.next? (238), insert? (251), emit Accepted? (256).
    #[test]
    fn replace_store_remove_err() {
        let seq = MockSequenceGenerator::new();
        let book = MockPriceBook::new();
        let mut store = MockOrderStore::new();
        store.expect_get().return_once(|&id| {
            Some(Order {
                id,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store
            .expect_remove()
            .return_once(|_| Err(OrderStoreError::NotFound(1)));
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let sink = MockEventSink::new();
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = ReplaceOrderCommand {
            order_id: 1,
            participant_id: 100,
            new_price: Some(50),
            new_quantity: 5,
            sequence: 0,
        };
        let res = engine.replace(cmd);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::OrderStore(_)));
    }

    #[test]
    fn replace_emit_canceled_err() {
        let seq = MockSequenceGenerator::new();
        let mut book = MockPriceBook::new();
        book.expect_remove().return_once(|_| true);
        let mut store = MockOrderStore::new();
        store.expect_get().return_once(|&id| {
            Some(Order {
                id,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store.expect_remove().return_once(|_| {
            Ok(Order {
                id: 1,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let mut sink = MockEventSink::new();
        sink.expect_emit().return_once(|e| match e {
            Event::Canceled(_) => Err(EventSinkError::Disconnected),
            _ => Ok(()),
        });
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = ReplaceOrderCommand {
            order_id: 1,
            participant_id: 100,
            new_price: Some(49),
            new_quantity: 5,
            sequence: 0,
        };
        let res = engine.replace(cmd);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::EventSink(_)));
    }

    #[test]
    fn replace_book_remove_none_invariant_violation() {
        let seq = MockSequenceGenerator::new();
        let mut book = MockPriceBook::new();
        book.expect_remove().return_once(|_| false);
        let mut store = MockOrderStore::new();
        store.expect_get().return_once(|&id| {
            Some(Order {
                id,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store.expect_remove().return_once(|_| {
            Ok(Order {
                id: 1,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let mut sink = MockEventSink::new();
        sink.expect_emit().returning(|_| Ok(()));
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = ReplaceOrderCommand {
            order_id: 1,
            participant_id: 100,
            new_price: Some(49),
            new_quantity: 5,
            sequence: 0,
        };
        let res = engine.replace(cmd);
        assert!(res.is_err());
        assert!(matches!(
            res.unwrap_err(),
            Error::Rejection(RejectionError::PriceBookInvariantViolation)
        ));
    }

    #[test]
    fn replace_sequence_err() {
        let mut seq = MockSequenceGenerator::new();
        let mut book = MockPriceBook::new();
        book.expect_remove().return_once(|_| true);
        let mut store = MockOrderStore::new();
        store.expect_get().return_once(|&id| {
            Some(Order {
                id,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store.expect_remove().returning(|_| {
            Ok(Order {
                id: 1,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store.expect_insert().returning(|_| Ok(()));
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let mut sink = MockEventSink::new();
        sink.expect_emit().returning(|_| Ok(()));
        seq.expect_next_at_end_of_queue()
            .return_once(|| Err(SequenceGeneratorError::Overflow));
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = ReplaceOrderCommand {
            order_id: 1,
            participant_id: 100,
            new_price: Some(50),
            new_quantity: 5,
            sequence: 0,
        };
        let res = engine.replace(cmd);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::Sequence(_)));
    }

    #[test]
    fn replace_store_insert_err() {
        let mut seq = MockSequenceGenerator::new();
        seq.expect_next_at_end_of_queue().return_once(|| Ok(1));
        let mut book = MockPriceBook::new();
        book.expect_pop_best().returning(|_| None);
        book.expect_remove().return_once(|_| true);
        let mut store = MockOrderStore::new();
        store.expect_get().return_once(|&id| {
            Some(Order {
                id,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store.expect_remove().returning(|_| {
            Ok(Order {
                id: 1,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store
            .expect_insert()
            .return_once(|_| Err(OrderStoreError::AlreadyExists(1)));
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let mut sink = MockEventSink::new();
        sink.expect_emit().returning(|_| Ok(()));
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = ReplaceOrderCommand {
            order_id: 1,
            participant_id: 100,
            new_price: Some(50),
            new_quantity: 5,
            sequence: 0,
        };
        let res = engine.replace(cmd);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::OrderStore(_)));
    }

    /// Cover replace: if let Some(price) = new.price { push } else (line 252-254 None branch).
    #[test]
    fn replace_new_price_none_skip_push() {
        let mut seq = MockSequenceGenerator::new();
        seq.expect_next_at_end_of_queue().return_once(|| Ok(1));
        let mut book = MockPriceBook::new();
        book.expect_pop_best().returning(|_| None);
        book.expect_remove().return_once(|_| true);
        let mut store = MockOrderStore::new();
        store.expect_get().return_once(|&id| {
            Some(Order {
                id,
                participant_id: 100,
                side: Side::Sell,
                order_type: OrderType::Market,
                price: None,
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store.expect_remove().returning(|_| {
            Ok(Order {
                id: 1,
                participant_id: 100,
                side: Side::Sell,
                order_type: OrderType::Market,
                price: None,
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store.expect_insert().returning(|_| Ok(()));
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let mut sink = MockEventSink::new();
        sink.expect_emit().returning(|_| Ok(()));
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = ReplaceOrderCommand {
            order_id: 1,
            participant_id: 100,
            new_price: None,
            new_quantity: 5,
            sequence: 0,
        };
        let res = engine.replace(cmd);
        assert!(res.is_ok());
    }

    #[test]
    fn replace_emit_accepted_err() {
        let mut seq = MockSequenceGenerator::new();
        seq.expect_next_at_end_of_queue().return_once(|| Ok(1));
        let mut book = MockPriceBook::new();
        book.expect_pop_best().returning(|_| None);
        book.expect_remove().return_once(|_| true);
        book.expect_push().returning(|_, _, _, _| {});
        let mut store = MockOrderStore::new();
        store.expect_get().return_once(|&id| {
            Some(Order {
                id,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store.expect_remove().returning(|_| {
            Ok(Order {
                id: 1,
                participant_id: 100,
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(50),
                quantity: 10,
                time_in_force: TimeInForce::Gtc,
                sequence: 0,
                leaves_quantity: 10,
                executed_quantity: 0,
                ..Order::default()
            })
        });
        store.expect_insert().returning(|_| Ok(()));
        let matching = MockMatchingPolicy::new();
        let self_trade = MockSelfTradePolicy::new();
        let mut sink = MockEventSink::new();
        sink.expect_emit().returning(|e| match e {
            Event::Accepted(_) => Err(EventSinkError::Disconnected),
            _ => Ok(()),
        });
        let mut engine =
            mock_engine(seq, book, store, matching, self_trade, sink);
        let cmd = ReplaceOrderCommand {
            order_id: 1,
            participant_id: 100,
            new_price: Some(50),
            new_quantity: 5,
            sequence: 0,
        };
        let res = engine.replace(cmd);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::EventSink(_)));
    }
}
