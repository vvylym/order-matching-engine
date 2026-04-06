//! ITCH handler integration test.
//!
//! Run parser + engine on sample.itch; assert message count and zero errors.
//! Sample file: `tests/fixtures/sample.itch` (or set `ITCH_SAMPLE_PATH`).

use std::io::BufReader;
use std::path::Path;

use omer::book::service::BTreeOrderBook;
use omer::engine::OrderMatchingEngine;
use omer::events::NoOpEventSink;
use omer::itch::process_itch_stream;
use omer::matching::PriceCrossMatchingPolicy;
use omer::self_trade::AllowAllSelfTradePolicy;
use omer::sequence::CounterSequenceGenerator;
use omer::store::service::HashMapOrderStore;

/// Builds ITCH 5.0 packet: 2-byte big-endian length, 1-byte type, payload.
fn itch_packet(msg_type: u8, payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u16;
    let mut out = Vec::with_capacity(3 + payload.len());
    out.extend_from_slice(&len.to_be_bytes());
    out.push(msg_type);
    out.extend_from_slice(payload);
    out
}

/// AddOrder payload (36 bytes): stock_locate@1(2), timestamp@5(6), oid@11(8), buy@19(1), qty@20(4), price@32(4).
fn add_order_payload(
    stock_locate: u16,
    oid: u64,
    buy: u8,
    qty: u32,
    price: u32,
) -> Vec<u8> {
    let mut p = vec![0u8; 36];
    p[1..3].copy_from_slice(&stock_locate.to_be_bytes());
    // 5..11 timestamp (0), 11..19 oid, 19 buy, 20..24 qty, 32..36 price
    p[11..19].copy_from_slice(&oid.to_be_bytes());
    p[19] = buy;
    p[20..24].copy_from_slice(&qty.to_be_bytes());
    p[32..36].copy_from_slice(&price.to_be_bytes());
    p
}

/// DeleteOrder payload (19 bytes): timestamp@5(6), oid@11(8).
fn delete_order_payload(oid: u64) -> Vec<u8> {
    let mut p = vec![0u8; 19];
    p[11..19].copy_from_slice(&oid.to_be_bytes());
    p
}

/// Returns the standard sample ITCH bytes: 2 AddOrders (bid 1, ask 2) then Delete 1.
/// Book-affecting messages: 3. Final state: one sell order (oid=2) at 10001.
fn sample_itch_bytes() -> Vec<u8> {
    let mut out = Vec::new();
    // AddOrder oid=1, buy, qty=10, price=10000, stock_locate=0
    out.extend(itch_packet(b'A', &add_order_payload(0, 1, b'B', 10, 10000)));
    // AddOrder oid=2, sell, qty=5, price=10001, stock_locate=0
    out.extend(itch_packet(b'A', &add_order_payload(0, 2, b'S', 5, 10001)));
    // DeleteOrder oid=1
    out.extend(itch_packet(b'D', &delete_order_payload(1)));
    out
}

/// Path to the sample.itch fixture (under crate manifest dir).
fn sample_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.itch")
}

#[test]
fn itch_process_sample_file_message_count_and_zero_errors() {
    let path = std::env::var("ITCH_SAMPLE_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| sample_fixture_path());

    let data = if path.exists() {
        std::fs::read(&path).expect("read sample.itch")
    } else {
        sample_itch_bytes()
    };

    let seq = CounterSequenceGenerator::new();
    let book = BTreeOrderBook::new();
    let store = HashMapOrderStore::new();
    let matching = PriceCrossMatchingPolicy;
    let self_trade = AllowAllSelfTradePolicy;
    let sink = NoOpEventSink;
    let mut engine =
        OrderMatchingEngine::new(seq, book, store, matching, self_trade, sink);

    let n = process_itch_stream(
        BufReader::new(std::io::Cursor::new(&data)),
        &mut engine,
    )
    .expect("ingestion must succeed with zero errors");

    assert_eq!(n, 3, "sample has 3 book-affecting messages");
    assert_eq!(engine.best_bid(), None, "bid order 1 was deleted");
    assert_eq!(
        engine.best_ask(),
        Some(10001),
        "ask order 2 remains at 10001"
    );
}

/// Writes `tests/fixtures/sample.itch` so it can be checked in. Run with:
/// `cargo test --test itch_handler write_sample_itch_fixture -- --ignored --exact`
#[test]
#[ignore = "run manually to regenerate tests/fixtures/sample.itch"]
fn write_sample_itch_fixture() {
    let path = sample_fixture_path();
    std::fs::create_dir_all(path.parent().unwrap()).expect("create fixtures dir");
    std::fs::write(&path, sample_itch_bytes()).expect("write sample.itch");
}
