//! Fully generic builder for [`OrderMatchingEngine`](crate::engine::OrderMatchingEngine).
//!
//! This builder is component-first: callers provide each concrete implementation
//! (sequence, book, store, matching policy, self-trade policy, sink), then call
//! `build()` once all components are present.
#![allow(clippy::type_complexity)]

use crate::book::PriceBook;
use crate::events::EventSink;
use crate::matching::MatchingPolicy;
use crate::self_trade::SelfTradePolicy;
use crate::sequence::SequenceGenerator;
use crate::store::OrderStore;

use super::OrderMatchingEngine;

/// Marker for a missing builder component.
#[derive(Debug, Clone, Copy, Default)]
pub struct Missing;

/// Marker for a configured builder component.
#[derive(Debug, Clone)]
pub struct Present<T>(pub T);

/// Generic builder for [`OrderMatchingEngine`].
#[derive(Debug, Clone)]
pub struct EngineBuilder<SG, PB, OS, MP, STP, ES> {
    sequence_generator: SG,
    price_book: PB,
    order_store: OS,
    matching_policy: MP,
    self_trade_policy: STP,
    event_sink: ES,
}

impl EngineBuilder<Missing, Missing, Missing, Missing, Missing, Missing> {
    /// Creates a new empty builder with all components missing.
    pub fn new() -> Self {
        Self {
            sequence_generator: Missing,
            price_book: Missing,
            order_store: Missing,
            matching_policy: Missing,
            self_trade_policy: Missing,
            event_sink: Missing,
        }
    }
}

impl Default
    for EngineBuilder<Missing, Missing, Missing, Missing, Missing, Missing>
{
    fn default() -> Self {
        Self::new()
    }
}

/// Starts building an [`OrderMatchingEngine`] by selecting each component.
pub fn builder()
-> EngineBuilder<Missing, Missing, Missing, Missing, Missing, Missing> {
    EngineBuilder::new()
}

impl<SG, PB, OS, MP, STP, ES> EngineBuilder<SG, PB, OS, MP, STP, ES> {
    /// Sets the sequence generator component.
    pub fn with_sequence_generator<NSG>(
        self,
        sequence_generator: NSG,
    ) -> EngineBuilder<Present<NSG>, PB, OS, MP, STP, ES> {
        EngineBuilder {
            sequence_generator: Present(sequence_generator),
            price_book: self.price_book,
            order_store: self.order_store,
            matching_policy: self.matching_policy,
            self_trade_policy: self.self_trade_policy,
            event_sink: self.event_sink,
        }
    }

    /// Sets the price book component.
    pub fn with_price_book<NPB>(
        self,
        price_book: NPB,
    ) -> EngineBuilder<SG, Present<NPB>, OS, MP, STP, ES> {
        EngineBuilder {
            sequence_generator: self.sequence_generator,
            price_book: Present(price_book),
            order_store: self.order_store,
            matching_policy: self.matching_policy,
            self_trade_policy: self.self_trade_policy,
            event_sink: self.event_sink,
        }
    }

    /// Sets the order store component.
    pub fn with_order_store<NOS>(
        self,
        order_store: NOS,
    ) -> EngineBuilder<SG, PB, Present<NOS>, MP, STP, ES> {
        EngineBuilder {
            sequence_generator: self.sequence_generator,
            price_book: self.price_book,
            order_store: Present(order_store),
            matching_policy: self.matching_policy,
            self_trade_policy: self.self_trade_policy,
            event_sink: self.event_sink,
        }
    }

    /// Sets the matching policy component.
    pub fn with_matching_policy<NMP>(
        self,
        matching_policy: NMP,
    ) -> EngineBuilder<SG, PB, OS, Present<NMP>, STP, ES> {
        EngineBuilder {
            sequence_generator: self.sequence_generator,
            price_book: self.price_book,
            order_store: self.order_store,
            matching_policy: Present(matching_policy),
            self_trade_policy: self.self_trade_policy,
            event_sink: self.event_sink,
        }
    }

    /// Sets the self-trade policy component.
    pub fn with_self_trade_policy<NSTP>(
        self,
        self_trade_policy: NSTP,
    ) -> EngineBuilder<SG, PB, OS, MP, Present<NSTP>, ES> {
        EngineBuilder {
            sequence_generator: self.sequence_generator,
            price_book: self.price_book,
            order_store: self.order_store,
            matching_policy: self.matching_policy,
            self_trade_policy: Present(self_trade_policy),
            event_sink: self.event_sink,
        }
    }

    /// Sets the event sink component.
    pub fn with_event_sink<NES>(
        self,
        event_sink: NES,
    ) -> EngineBuilder<SG, PB, OS, MP, STP, Present<NES>> {
        EngineBuilder {
            sequence_generator: self.sequence_generator,
            price_book: self.price_book,
            order_store: self.order_store,
            matching_policy: self.matching_policy,
            self_trade_policy: self.self_trade_policy,
            event_sink: Present(event_sink),
        }
    }
}

impl<SG, PB, OS, MP, STP, ES>
    EngineBuilder<
        Present<SG>,
        Present<PB>,
        Present<OS>,
        Present<MP>,
        Present<STP>,
        Present<ES>,
    >
where
    SG: SequenceGenerator,
    PB: PriceBook,
    OS: OrderStore,
    MP: MatchingPolicy,
    STP: SelfTradePolicy,
    ES: EventSink,
{
    /// Builds the engine once all components are provided.
    pub fn build(self) -> OrderMatchingEngine<SG, PB, OS, MP, STP, ES> {
        OrderMatchingEngine::new(
            self.sequence_generator.0,
            self.price_book.0,
            self.order_store.0,
            self.matching_policy.0,
            self.self_trade_policy.0,
            self.event_sink.0,
        )
    }
}
