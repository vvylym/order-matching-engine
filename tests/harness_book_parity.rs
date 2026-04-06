//! Harness book backends must all implement matching-compatible [`PriceBook::pop_best`] (aggressor
//! side consumes opposite book). Same scenario as `marketable_limit_matches_immediately` for each
//! [`omer::harness`] book constructor.

use omer::engine::OrderMatchingService;
use omer::events::Event;
use omer::harness::{
    add_cmd, engine_with_btree_book, engine_with_dash_skip_book,
    engine_with_memory, engine_with_pool_level_book,
};
use omer::types::{OrderType, Side, TimeInForce};

macro_rules! harness_marketable_buy_fills_sell {
    ($test_name:ident, $make_engine:path) => {
        #[test]
        fn $test_name() {
            let (mut engine, sink) = $make_engine();
            engine
                .add(add_cmd(
                    1,
                    100,
                    Side::Sell,
                    OrderType::Limit,
                    Some(50),
                    10,
                    TimeInForce::Gtc,
                ))
                .unwrap();
            sink.clear();
            engine
                .add(add_cmd(
                    2,
                    100,
                    Side::Buy,
                    OrderType::Limit,
                    Some(55),
                    10,
                    TimeInForce::Gtc,
                ))
                .unwrap();
            let ev = sink.events();
            assert_eq!(ev.len(), 1, "full fill emits one trade");
            assert!(matches!(ev[0], Event::Trade(_)));
        }
    };
}

harness_marketable_buy_fills_sell!(marketable_buy_inmemory, engine_with_memory);
harness_marketable_buy_fills_sell!(
    marketable_buy_btree_book,
    engine_with_btree_book
);
harness_marketable_buy_fills_sell!(
    marketable_buy_pool_level_book,
    engine_with_pool_level_book
);
harness_marketable_buy_fills_sell!(
    marketable_buy_dash_skip_book,
    engine_with_dash_skip_book
);
