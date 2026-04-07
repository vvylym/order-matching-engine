//! Deep ITCH fixture coverage and end-to-end matching throughput are still growing.
//! These tests only guard that ITCH/matching bench targets stay registered in `Cargo.toml` and README.

#[test]
fn cargo_toml_lists_bench_targets() {
    let cargo = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    for name in [
        "matching_engine",
        "micro",
        "market_manager",
        "latency_add",
        "latency_cancel",
        "latency_replace",
        "latency_market",
        "itch_parse",
        "throughput_book",
        "throughput_engine",
        "throughput_mixed",
        "throughput_adversarial",
        "correctness",
        "memory_hot_path",
        "integrity_stress",
        "observability_overhead",
        "parallel_best_quotes",
    ] {
        assert!(
            cargo.contains(name),
            "Cargo.toml should keep [[bench]] entry for {name}"
        );
    }
}

#[test]
fn readme_documents_itch_entry_point() {
    let readme = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"));
    assert!(
        readme.contains("itch") || readme.contains("ITCH"),
        "README should mention the ITCH module"
    );
}
