//! **MarketManager** / multi-instrument scenarios are not implemented in this crate yet.
//! This test only checks that `benches/PLAN.md` still lists the planned keywords.

#[test]
fn bench_plan_covers_latency_and_scenario_backlog() {
    let plan =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/benches/PLAN.md"));
    for needle in [
        "latency_add",
        "throughput_mixed",
        "correctness",
        "multi-instrument",
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
