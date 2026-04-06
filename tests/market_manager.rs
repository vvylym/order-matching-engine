//! CppTrader-style **MarketManager** scenarios are not implemented in this crate yet.
//! This module keeps **enabled** tests that only verify the written roadmap stays intact.

#[test]
fn bench_plan_covers_latency_and_scenario_backlog() {
    let plan =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/benches/PLAN.md"));
    for needle in [
        "latency_add",
        "throughput_mixed",
        "correctness",
        "CppTrader",
        "IOC",
        "FOK",
        "AON",
        "iceberg",
        "stop-limit",
        "trailing",
        "MarketManager",
    ] {
        assert!(
            plan.contains(needle),
            "benches/PLAN.md should mention `{needle}` for traceability"
        );
    }
}
