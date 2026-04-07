# omer — order matching engine

[![CI](https://github.com/vvylym/order-matching-engine/actions/workflows/ci.yml/badge.svg)](https://github.com/vvylym/order-matching-engine/actions/workflows/ci.yml)

Rust library: **limit/market** matching on a **price–time** book. Commands (`add`, `cancel`, `replace`, …) go through **`OrderMatchingEngine`**. Numeric fields use integers (ticks / lots); no `f64` on the hot path.

**Repo:** [github.com/vvylym/order-matching-engine](https://github.com/vvylym/order-matching-engine) · **crate:** `omer`

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
omer = "0.1"
```

Optional: **`parallel`** (pulls in `rayon` for read-mostly helpers):

```toml
omer = { version = "0.1", features = ["parallel"] }
```

Omit default harness for library-only use:

```toml
omer = { version = "0.1", default-features = false }
```

---

## Usage

```rust
use omer::engine::{OrderMatchingService, OrderCommand};
use omer::harness::{add_cmd, engine_with_memory};
use omer::types::{OrderType, Side, TimeInForce};

let (mut engine, _sink) = engine_with_memory();
let cmd = OrderCommand::Add(add_cmd(
    1,
    100,
    Side::Buy,
    OrderType::Limit,
    Some(50),
    10,
    TimeInForce::Gtc,
));
engine.process(cmd).expect("accepted");
```

See [`tests/`](tests/) and [`src/engine/models.rs`](src/engine/models.rs) for the full command set.

---

## Repository layout

| Path | Role |
|------|------|
| [`src/lib.rs`](src/lib.rs) | Crate root; module overview |
| [`src/types.rs`](src/types.rs) | `Order`, `Price`, `Quantity`, sides, TIF |
| [`src/engine/`](src/engine/) | `OrderMatchingService`, `OrderMatchingEngine`, commands |
| [`src/book/`](src/book/) | [`PriceBook`](src/book/mod.rs) trait + implementations |
| [`src/store/`](src/store/) | [`OrderStore`](src/store/mod.rs) trait + implementations |
| [`src/events/`](src/events/) | `Event`, `EventSink`, `NoOpEventSink` |
| [`src/matching/`](src/matching/), [`src/self_trade/`](src/self_trade/), [`src/sequence/`](src/sequence/) | Policies and sequence generation |
| [`src/itch/`](src/itch/) | ITCH-style decode + streaming entry points |
| [`src/pool/`](src/pool/mod.rs) | `OrderPool` (reuse `Order` shells outside the engine) |
| [`src/parallel.rs`](src/parallel.rs) | Optional `rayon` helper for read-mostly aggregation (feature **`parallel`**) |
| [`src/harness/`](src/harness/) | Default test/bench wiring (feature **`harness`**, on by default) |
| [`benches/`](benches/) | Criterion targets (`--no-run` in CI) |
| [`tests/`](tests/) | Integration and property tests |

Disable harness: `omer = { version = "…", default-features = false }`.

---

## Services (traits) and implementations

| Service | Trait | Concrete types |
|---------|--------|----------------|
| Matching entrypoint | [`OrderMatchingService`](src/engine/service.rs) | [`OrderMatchingEngine`](src/engine/service.rs) |
| Resting book | [`PriceBook`](src/book/mod.rs) | [`BTreeOrderBook`](src/book/service/btree.rs), [`PoolLevelOrderBook`](src/book/service/pool_level.rs), [`DashSkipOrderBook`](src/book/service/dash_skip.rs), [`InMemoryPriceBook`](src/harness/memory.rs) (harness) |
| Order storage | [`OrderStore`](src/store/mod.rs) | [`HashMapOrderStore`](src/store/service/hash_map.rs), [`DenseOrderStore`](src/store/service/dense.rs), [`InMemoryOrderStore`](src/harness/memory.rs) |

**Resting layout (important):** the book keeps **`OrderId` queues per price**; the **canonical [`Order`] payload** lives only in **`OrderStore`**. `PriceBook::push` takes `order_id`, `side`, and a **`time_priority`** (`Sequence`) so the in-memory test book can respect time priority; FIFO backends ignore that field.

**Pooling:** [`OrderPool`](src/pool/mod.rs) recycles `Order` structs for adapters and gateways (not required for core matching).

**Parallel reads:** enable `parallel` and use [`parallel::par_best_quotes`](src/parallel.rs) to aggregate best bid/ask over many books in parallel; matching remains single-writer per book.

---

## Benchmarks (what they measure)

| Bench | Command | What it does |
|-------|---------|----------------|
| `throughput_engine` | `cargo bench -p omer --bench throughput_engine` | Warm book + `process_batch` (512 GTC buys), **four** `PriceBook` backends, `NoOpEventSink` |
| `latency_add` | `cargo bench -p omer --bench latency_add` | One resting `add` per iteration, same four backends, collecting sink |
| `throughput_book` | `cargo bench -p omer --bench throughput_book` | `DashSkipOrderBook::push` only (ids; no engine) |
| `itch_parse` | `cargo bench -p omer --bench itch_parse` | `scan_decode_book_messages` on AddOrder buffer |
| `micro`, `matching_engine`, `market_manager` | same pattern | Smoke / placeholder loops; see [`benches/PLAN.md`](benches/PLAN.md) |

CI compiles all benches with `cargo bench --no-run --workspace --all-features` but does not report Criterion output.

---

## Profiling (engine hot path)

Release build, pinned CPU, frame pointers:

```bash
export RUSTFLAGS="-C force-frame-pointers=yes"
cargo flamegraph --bench throughput_engine --features harness -- --bench
```

Alternatives: `perf record` + flame graph tooling (see earlier commits or team runbooks). Attach machine type, `rustc -V`, and git SHA with any numbers.

---

## Quick start

```bash
cargo test --all-features --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all
```

Coverage (optional): [`scripts/coverage.sh`](scripts/coverage.sh) (CI enforces line coverage threshold).

---

## Contributing and quality gates

See [`CONTRIBUTING.md`](CONTRIBUTING.md): **issue → branch → incremental commits → PR → review**. Use `make ci` to mirror CI and `make quality-gate` for optional [`pmat`](https://crates.io/crates/pmat) checks (local-only; see `Makefile`).

---

## License

[MIT](LICENSE).
