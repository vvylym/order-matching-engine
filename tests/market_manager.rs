//! Market manager scenario tests (CppTrader parity).
//!
//! Each test encodes one scenario from the plan. They will not compile until
//! MarketManager, SymbolId, Symbol, extended OrderType/TimeInForce and book/event
//! APIs exist. Once implemented, remove #[ignore] and assert exact book/event outcomes.

// Target: omer::manager::MarketManager, omer::types::{SymbolId, Symbol}, extended Order/OrderType/TimeInForce.

#[test]
#[ignore = "full_port: scenario market order"]
fn scenario_market_order() {
    // Add limit orders both sides, then add sell market order → match against best bids.
    // Assert: book volumes and order counts after each step; slippage (e.g. only fill at best).
}

#[test]
#[ignore = "full_port: scenario limit order"]
fn scenario_limit_order_crossing() {
    // Add limit orders; add crossing limit orders; assert matching and resting quantities per level.
    // Test modify and replace that change book state.
}

#[test]
#[ignore = "full_port: scenario IOC"]
fn scenario_ioc_limit_order() {
    // Add IOC limit order; assert full cancel if no fill or partial fill + cancel remainder.
}

#[test]
#[ignore = "full_port: scenario FOK filled"]
fn scenario_fok_filled() {
    // FOK that can be fully filled; assert book state after fill.
}

#[test]
#[ignore = "full_port: scenario FOK killed"]
fn scenario_fok_killed() {
    // FOK that cannot be fully filled; assert entire order cancelled, book unchanged.
}

#[test]
#[ignore = "full_port: scenario AON full matching"]
fn scenario_aon_full_matching() {
    // Several levels with AON orders; add AON that fills in one shot; assert full fill.
}

#[test]
#[ignore = "full_port: scenario AON partial"]
fn scenario_aon_partial_matching() {
    // AON that cannot fill fully; assert no fill, resting state.
}

#[test]
#[ignore = "full_port: scenario AON complex"]
fn scenario_aon_complex_matching() {
    // Mixed AON and regular orders; one AON triggers full match; assert final book.
}

#[test]
#[ignore = "full_port: scenario hidden/iceberg"]
fn scenario_hidden_iceberg_limit_order() {
    // Add limit orders with max_visible_quantity; run market order; assert visible vs total volume.
}

#[test]
#[ignore = "full_port: scenario stop order"]
fn scenario_stop_order() {
    // Add limit orders, then stop orders; trigger by crossing stop price; assert stop→market fill.
}

#[test]
#[ignore = "full_port: scenario stop order empty market"]
fn scenario_stop_order_empty_market() {
    // Stop with empty book; assert no fill, no panic.
}

#[test]
#[ignore = "full_port: scenario stop-limit"]
fn scenario_stop_limit_order() {
    // Add stop-limit; trigger; assert conversion to limit and fill/rest.
}

#[test]
#[ignore = "full_port: scenario stop-limit empty market"]
fn scenario_stop_limit_empty_market() {
    // Stop-limit with empty market; assert rest or no fill.
}

#[test]
#[ignore = "full_port: scenario trailing stop"]
fn scenario_trailing_stop() {
    // Set up market, add trailing stop; move market (modify best bid/ask); assert trailing stop price updates and activation.
}

#[test]
#[ignore = "full_port: scenario in-flight mitigation"]
fn scenario_in_flight_mitigation() {
    // Add orders, partial fill; mitigate with new price/quantity; assert leaves and executed quantities.
}

#[test]
#[ignore = "full_port: scenario manual matching"]
fn scenario_manual_matching() {
    // Add bids and asks, auto_matching off; call match_(); assert crossed volume matched, remaining book.
}
