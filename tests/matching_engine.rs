//! Matching engine + ITCH integration test.
//!
//! Run ITCH with matching enabled; assert final book/order stats in the same
//! ballpark as C++ (max symbols, max orders, add/update/delete/execute counts).
//! Will not compile until MarketManager + ITCH integration exists.

#[test]
#[ignore = "full_port: matching engine + ITCH not yet implemented"]
fn itch_with_matching_produces_expected_stats() {
    // let path = std::env::var("ITCH_SAMPLE_PATH").unwrap_or_else(|_| "tools/itch/sample.itch".to_string());
    // let data = std::fs::read(&path).unwrap();
    // let manager = MarketManager::new(...).with_matching_enabled(true);
    // let handler = DefaultItchHandler::new(&mut manager);
    // omer::itch::process(&data, &mut handler);
    // Assert: manager stats (max_symbols, max_order_books, max_orders, add_orders, update_orders, delete_orders, execute_orders)
    // in same ballpark as CppTrader test (e.g. max_symbols ~8352, max_orders ~56245, etc.)
}
