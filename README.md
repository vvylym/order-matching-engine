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

### Generic builder composition

You can compose the engine from concrete component types with `omer::engine::builder()`:

```rust
use omer::book::service::BTreeOrderBook;
use omer::engine::{builder, OrderMatchingService};
use omer::events::NoOpEventSink;
use omer::matching::PriceCrossMatchingPolicy;
use omer::self_trade::AllowAllSelfTradePolicy;
use omer::sequence::CounterSequenceGenerator;
use omer::store::service::HashMapOrderStore;

let mut engine = builder()
    .with_sequence_generator(CounterSequenceGenerator::new())
    .with_price_book(BTreeOrderBook::new())
    .with_order_store(HashMapOrderStore::new())
    .with_matching_policy(PriceCrossMatchingPolicy)
    .with_self_trade_policy(AllowAllSelfTradePolicy)
    .with_event_sink(NoOpEventSink)
    .build();
```

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
| `throughput_engine` | `cargo bench -p omer --features harness --bench throughput_engine` | Warm book + `process_batch` (512 GTC limit buys), four `PriceBook` backends, `NoOpEventSink` |
| `throughput_mixed` | `cargo bench -p omer --features harness --bench throughput_mixed` | 512-op round: mostly adds, cancels, occasional resting sell + IOC buy; `InMemoryPriceBook` |
| `throughput_adversarial` | `cargo bench -p omer --features harness --bench throughput_adversarial` | 5k distinct sell levels on `DashSkipOrderBook` + 256 market IOC sweeps per iteration |
| `throughput_book` | `cargo bench -p omer --bench throughput_book` | `DashSkipOrderBook::push` only (50k ids); FIFO at one price vs distinct prices |
| `latency_add` | `cargo bench -p omer --features harness --bench latency_add` | One resting `add` per iteration, four backends, `NoOpEventSink` |
| `latency_cancel` | `cargo bench -p omer --features harness --bench latency_cancel` | Add + `cancel_by_order_id` on that id each iteration (cycle cost) |
| `latency_replace` | `cargo bench -p omer --features harness --bench latency_replace` | Add + `replace` each iteration |
| `latency_market` | `cargo bench -p omer --features harness --bench latency_market` | Resting limit sell + IOC market buy cross each iteration |
| `correctness` | `cargo bench -p omer --bench correctness` | 256× minimal-engine replays + top-of-book checksum xor per Criterion iter |
| `memory_hot_path` | `cargo bench -p omer --bench memory_hot_path` | `allocation_counter::measure` around warm add+cancel on stable id (wall time + alloc hook) |
| `integrity_stress` | `cargo bench -p omer --bench integrity_stress` | Deterministic mixed ops + `assert_uncrossed` (Criterion needs `--sample-size` ≥ 10) |
| `observability_overhead` | `cargo bench -p omer --features harness --bench observability_overhead` | Same add pattern: `NoOpEventSink` vs collecting harness sink |
| `parallel_best_quotes` | `cargo bench -p omer --features parallel --bench parallel_best_quotes` | 256 `BTreeOrderBook` instances: sequential best-quote scan vs `par_best_quotes` |
| `throughput_sharded_add` | `cargo bench -p omer --features harness,parallel --bench throughput_sharded_add` | Sharded add-only workload: one engine per shard, adds executed in parallel (`rayon`) |
| `throughput_sharded_book_push` | `cargo bench -p omer --features parallel --bench throughput_sharded_book_push` | Sharded book-only `DashSkipOrderBook::push` in parallel; same-price FIFO vs distinct prices |
| `throughput_sharded_mixed` | `cargo bench -p omer --features harness,parallel --bench throughput_sharded_mixed` | Sharded mixed flow with explicit `OrderId -> shard` index for cancel routing |
| `lock_read_heavy` | `cargo bench -p omer --bench lock_read_heavy` | Read-heavy lock comparison: `Arc<std::sync::RwLock<_>>` vs `Arc<parking_lot::RwLock<_>>` vs `Arc<tokio::sync::RwLock<_>>` |
| `itch_parse` | `cargo bench -p omer --bench itch_parse` | `scan_decode_book_messages` on 50k AddOrder packets in one buffer |
| `micro` | `cargo bench -p omer --bench micro` | Near-no-op Criterion smoke (picosecond-scale; not engine work) |
| `matching_engine`, `market_manager` | same pattern | ITCH-shaped smoke benches |

CI compiles benches with `cargo bench --no-run --workspace --all-features` but does not publish Criterion HTML.

### Observed results (reference machine)

One **release** run on **Linux 6.5**, **rustc 1.94.1**, **Intel i7-13650HX (20 CPUs)**, with Criterion short sampling (`--sample-size 10`, ~0.15–0.25 s measurement). Values are **medians** (middle of Criterion’s printed `[lower, estimate, upper]` interval)—use for regression trending, not as absolute guarantees.

| Bench / function | Median | Notes |
|------------------|--------|--------|
| `itch_parse` / 50k add messages | **~191 µs** | ~262M decoded msgs/s of this fixture shape |
| `throughput_book` / same-price FIFO, 50k `push` | **~2.15 ms** | ~23M pushes/s |
| `throughput_book` / distinct prices, 50k `push` | **~10.6 ms** | ~4.7M pushes/s |
| `throughput_engine` / `process_batch` 512 ops, inmemory | **~171 µs** | ~3.0M engine adds/s (batch) |
| `throughput_engine` / btree | **~247 µs** | |
| `throughput_engine` / pool_level | **~264 µs** | |
| `throughput_engine` / dash_skip | **~131 µs** | |
| `latency_add` / inmemory … dash_skip | **~350 … ~512 ns** | Resting limit add, noop sink |
| `latency_cancel` / inmemory … dash_skip | **~72 … ~254 ns** | Full add+cancel cycle |
| `latency_replace` / inmemory … dash_skip | **~257 … ~522 ns** | Add+replace |
| `latency_market` / inmemory … dash_skip | **~111 … ~285 ns** | Limit sell + IOC buy cross |
| `throughput_mixed` / 512-op round | **~41 µs** | ~12.5M “elements”/s (Criterion throughput tag) |
| `throughput_adversarial` / 5256 ops | **~1.35 ms** | Deep book + sweeps |
| `correctness` / 256 replays | **~88.8 µs** | Per-iter cost of 256× `replay_once` |
| `memory_hot_path` / instrumented add+cancel | **~100 ns** | Wall time under `allocation-counter`; see bench source for alloc semantics |
| `integrity_stress` / randomish stream | **~3.75 ms** | Per iteration; includes invariant check |
| `observability_overhead` / noop vs collecting add | **~189 ns vs ~141 ns** | Overlapping CIs—treat delta as noisy at this sampling depth |
| `parallel_best_quotes` / sequential vs `rayon` | **~710 ns vs ~19.9 µs** | 256 tiny books: parallel fork/join dominates; scale up book count or work per book to see win |
| `throughput_sharded_add` / 20 shards add-only | **~19.1 Melem/s** | Aggregate across shards; still far from 1B ops/s without further batching/layout work |
| `throughput_sharded_book_push` / 20 shards same-price FIFO | **~57.8 Melem/s** | Upper bound (book-only). Distinct prices median was **~23.8 Melem/s** |
| `throughput_sharded_mixed` / 20 shards + `OrderId -> shard` index | **~2.39 Melem/s** | More realistic mixed path; routing lookups + mixed command costs are visible |
| `lock_read_heavy` / std vs parking_lot vs tokio RwLock | **~54.5 / ~52.7 / ~25.9 Melem/s** | This synthetic read-heavy test favors sync locks; async lock costs more per op |

`micro` / `matching_engine` / `market_manager` report **~210 ps** per iter (empty loops)—kept as compile/smoke anchors only.

### Flamegraph priority rollout log (ordered steps)

Baseline commands used before Priority 1 (kept fixed for comparison):

- `cargo bench --features harness,parallel --bench throughput_sharded_mixed -- --sample-size 10 --warm-up-time 1 --measurement-time 3`
- `cargo bench --bench lock_read_heavy -- --sample-size 10 --warm-up-time 1 --measurement-time 3`

Baseline medians from those commands:

- `throughput_sharded_mixed`: about **470,480 operations per second**
- `lock_read_heavy`:
  - `std::sync::RwLock`: about **59,177,000 operations per second**
  - `parking_lot::RwLock`: about **52,860,000 operations per second**
  - `tokio::sync::RwLock`: about **24,760,000 operations per second**

#### Priority 1 result: indexed remove in `InMemoryPriceBook`

- Before: about **470,480 operations per second** (`throughput_sharded_mixed`)
- After: about **2,446,400 operations per second** (`throughput_sharded_mixed`)
- Relative change: roughly **5.2x higher throughput**

What we tried:

- Added an internal `order_id -> (side, price)` index in the harness in-memory book.
- Kept public `PriceBook` behavior and trait interface unchanged.

What worked:

- `remove` now looks up the target level directly and scans one queue instead of all book levels.
- Hot path throughput improved strongly on the sharded mixed workload.

What did not work:

- Nothing functionally regressed in tests, but this does not remove the per-level queue scan yet (`VecDeque::position` is still linear within one price level).

Tradeoffs:

- Slightly more memory and write-time bookkeeping (maintaining the index) in exchange for much faster cancel/remove routing in mixed workloads.

---

## Tokio harness binaries

This repo now includes a lightweight benchmark harness server and client:

- `cargo run --bin server -- --bind 127.0.0.1:7001 --shards 8`
- `cargo run --bin client -- --addr 127.0.0.1:7001 --connections 4 --symbols 32 --duration-secs 8`

Protocol is intentionally simple for measurement (not production API):

- `ADD <id> <participant> <symbol> <B|S> <price> <qty>`
- `MARKET <id> <participant> <symbol> <B|S> <qty>`
- `CANCELID <order_id>`

Reference run on the same machine:

- `connections=4 duration_s=8 ok_ops=761874 err_ops=0 throughput_ops_s=95234.25 avg_latency_ns=41537.87`

---

## Profiling (engine hot path)

Release build, pinned CPU, frame pointers:

```bash
export RUSTFLAGS="-C force-frame-pointers=yes"
cargo flamegraph -p omer --bench throughput_engine --features harness -- --bench
```

For the sharded mixed path, a sample flamegraph artifact is included at:

- [`docs/perf/flamegraph-throughput_sharded_mixed.svg`](docs/perf/flamegraph-throughput_sharded_mixed.svg)

Command used:

```bash
export RUSTFLAGS="-C force-frame-pointers=yes"
cargo flamegraph -p omer --bench throughput_sharded_mixed --features harness,parallel --output docs/perf/flamegraph-throughput_sharded_mixed.svg -- --bench --noplot --sample-size 10 --warm-up-time 0.1 --measurement-time 0.2
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
