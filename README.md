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
| `throughput_sharded_mixed` | `cargo bench -p omer --features harness,parallel --bench throughput_sharded_mixed` | Sharded mixed flow with explicit `OrderId -> shard` index for cancel routing, measured on `InMemoryPriceBook`, `BTreeOrderBook`, and `DashSkipOrderBook` |
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
| `throughput_sharded_add` / 20 shards add-only | **~19,100,000 operations per second** | Aggregate across shards; still far from 1B ops/s without further batching/layout work |
| `throughput_sharded_book_push` / 20 shards same-price FIFO | **~57,800,000 operations per second** | Upper bound (book-only). Distinct prices median was **~23,800,000 operations per second** |
| `throughput_sharded_mixed` / 20 shards + `OrderId -> shard` index | **~2,390,000 operations per second** | More realistic mixed path; routing lookups + mixed command costs are visible |
| `lock_read_heavy` / std vs parking_lot vs tokio RwLock | **~54,500,000 / ~52,700,000 / ~25,900,000 operations per second** | This synthetic read-heavy test favors sync locks; async lock costs more per op |

`micro` / `matching_engine` / `market_manager` report **~210 ps** per iter (empty loops)—kept as compile/smoke anchors only.

### Where we are now (clear answer)

Best measured combinations on this machine (using current benchmark suite):

- **Highest raw book throughput (book-only upper bound):**
  - `DashSkipOrderBook` on `throughput_sharded_book_push` same-price case: about **57,800,000 operations per second**
- **Best mixed-workload throughput (more realistic flow):**
  - `InMemoryPriceBook` on `throughput_sharded_mixed`: about **2,295,600 to 2,446,400 operations per second** after Priority 1-4 changes
- **Best single-operation latency among book backends in listed runs:**
  - `latency_cancel` fastest value: about **72 nanoseconds**
  - `latency_add` fastest value: about **350 nanoseconds**
  - `latency_market` fastest value: about **111 nanoseconds**
- **Memory hot-path indicator:**
  - `memory_hot_path` stayed around **100 nanoseconds** wall time in the instrumented add+cancel benchmark; use the benchmark source as the truth for allocation semantics.

Current measured status on this machine:

- Current mixed throughput is around **2.3 to 2.4 million operations per second**.
- The realistic path still needs algorithm and data-structure improvements for higher sustained throughput.

### Deployment-style quick answer

If you want one practical answer for this machine:

- For a realistic mixed workload, the current measured envelope is about **2.3 to 2.4 million orders per second**.
- At **2.3 million orders/second**:
  - ingesting **700,000,000** orders takes about **304,000 ms** (about **5.1 minutes**),
  - ingesting **1,000,000,000** orders takes about **435,000 ms** (about **7.2 minutes**).

For context only (less realistic upper bounds from narrower benchmarks):

- Book-only push path peak around **57.8 million orders/second** would imply about **12,100 ms** for 700,000,000 orders.
- Sharded add-only path around **19.1 million orders/second** would imply about **36,600 ms** for 700,000,000 orders.

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

#### Priority 2 result: reduce routing/index lookup overhead

- Before: about **2,446,400 operations per second** (`throughput_sharded_mixed`, after Priority 1)
- After: about **2,043,200 operations per second**
- Relative change: roughly **16% lower throughput** in this short run (Criterion reported no statistically significant change)

What we tried:

- Removed an unnecessary map-presence check in the sharded mixed benchmark cancel path.
- Changed server cancel routing from `read + get` to a single `write + remove` pass, which also cleans up stale route entries.

What worked:

- Server-side cancel routing is now cleaner and does one map operation while removing canceled IDs from the route index.

What did not work:

- This isolated step did not produce a clear throughput gain in the benchmark; measured result trended lower in this sample window.

Tradeoffs:

- Better route-index lifecycle behavior and simpler fast path, but no immediate benchmark win at this sampling depth.

#### Priority 3 result: sharded mixed benchmark on multiple backends

Median throughput from the same fixed benchmark command:

- `InMemoryPriceBook`: about **2,379,900 operations per second**
- `BTreeOrderBook`: about **1,565,500 operations per second**
- `DashSkipOrderBook`: about **1,587,900 operations per second**

What we tried:

- Generalized the sharded mixed benchmark to run the same workload against three book backends.

What worked:

- We now get direct backend-to-backend numbers under one benchmark harness and one command.
- The in-memory harness backend remained the fastest in this mixed test shape.

What did not work:

- This step is measurement-focused; it does not by itself improve throughput.

Tradeoffs:

- Longer benchmark runtime (three benchmark functions instead of one) in exchange for clearer backend parity data.

#### Priority 4 result: reuse command batches and reduce allocation churn

Before/after medians from the same benchmark command:

- `InMemoryPriceBook`: **2,379,900 -> 2,295,600 operations per second**
- `BTreeOrderBook`: **1,565,500 -> 1,748,700 operations per second**
- `DashSkipOrderBook`: **1,587,900 -> 1,613,500 operations per second**

What we tried:

- Reused per-shard command vectors across rounds (`clear` + `drain`) instead of creating fresh vectors every round.
- Pre-sized per-shard alive queues and command buffers with `OPS_PER_SHARD` capacity.

What worked:

- Allocation behavior is now steadier in the benchmark loop (capacity is retained across rounds).

What did not work:

- Throughput differences were mixed and not statistically significant in this short sample run.

Tradeoffs:

- Slightly more complex benchmark code for buffer lifecycle management, with no clear immediate throughput win.

#### Priority 5 result: cleaner profiling workflow and artifacts

What we tried:

- Standardized profiling with frame pointers, bench debug symbols, CPU pinning, and `--noplot`.
- Added a focused artifact for one benchmark function:
  - `docs/perf/flamegraph-throughput_sharded_mixed-inmemory-clean.svg`

What worked:

- The new command is reproducible and easier to rerun with the same context.
- Focusing on one function at a time reduces unrelated benchmark noise in flamegraph review.

What did not work:

- On this Linux setup, `perf` still reported restricted kernel symbols and one dropped chunk, so some low-level frames can remain unresolved.

Tradeoffs:

- Cleaner flamegraphs require stricter run conditions (core pinning, fewer background tasks), which can make absolute throughput values from that run less representative of normal multi-core throughput.

#### Priority 6 result: distributed multi-instrument protocol foundation (`p1`)

What we tried:

- Introduced a shared typed wire protocol module in `src/distributed_wire.rs` used by both harness binaries.
- Added explicit instrument-aware routing in the server with one matcher worker per instrument (`--instruments`, default 4).
- Switched client emission to instrument-aware frames with configurable batch size (`--batch-size`) and optional `BATCH` line frames.

What worked:

- Gateway parse/validation now rejects malformed lines and invalid instrument IDs before dispatch.
- Routing is deterministic (`instrument_id -> dedicated worker`) and cancel commands validate route consistency (`order_id` index + instrument match).
- Matcher boundary checks ensure worker and command instrument IDs align before processing.

What did not work:

- This step establishes architecture/validation boundaries only; no throughput tuning was targeted in this priority.
- Legacy ad-hoc line parsing path was removed, so older `CANCELID <order_id>` payloads must include instrument (`CANCELID <order_id> <instrument_id>`).

Tradeoffs:

- More protocol strictness and explicit routing metadata improve correctness and diagnosability, at the cost of slightly more wire payload and parsing logic.

#### Priority 7 result: deeper remove-path indexing candidate (`p2`)

What we tried:

- Reworked harness in-memory price levels to use an indexed level queue (`order_id -> position`) instead of linear `position` scans on cancel/remove.
- Kept external `PriceBook` behavior unchanged while tightening the remove hot path internals.

What worked:

- Remove-by-id now resolves directly through per-level index metadata and avoids repeated queue scans.
- CI, quality gate, and full test suite remained green with no correctness regression.

What did not work:

- In this short benchmark window, mixed throughput did not show a statistically significant win (`No change in performance detected`).

Tradeoffs:

- The indexed queue carries extra bookkeeping (index updates on swaps/removals) and higher implementation complexity.
- Keep this as a measured candidate rather than a guaranteed throughput improvement across environments.

#### Priority 8 result: cache-density pass on harness level queues (`p3`)

What we tried:

- Stored per-level slot indices as `u32` instead of `usize` in the `order_id -> slot` map to shrink value size on 64-bit targets.
- Reordered resting-queue tuples to `(Sequence, OrderId)` so `pop_min_sequence` scans compare the time-priority key before the order id.
- Documented harness invariant: at most `u32::MAX` resting orders per single price level (above that the harness panics with a clear message).

What worked:

- Keeps `PriceBook` behavior identical while narrowing hot map values and improving scan locality for sequence selection.
- `make ci`, `make quality-gate`, and the fixed `throughput_sharded_mixed` subset all passed on this machine.

Measured subset (same command as other rollout steps: `throughput_sharded_mixed` / `inmemory`, sample-size 10, short window):

- Throughput interval about **5.40M to 7.49M elements/s** in this run (Criterion printed `[5.4018, 6.3373, 7.4948] Melem/s`).

What did not work:

- This short window is noisy versus the prior `p2` run on the same command; treat the interval as a single observation, not a proven regression or win versus `p3` baseline without more repeats.

Tradeoffs:

- Extra invariant (`u32` level capacity) is acceptable for the benchmark harness but would need a different strategy for a production book that must support extreme depth per level.
- `total_depth` now walks levels with explicit loops so `FnMut` resolution stays correct under clippy; slightly more code for the same semantics.

#### Priority 9 result: `SmallVec` on distributed wire path (`p4`)

What we tried:

- Added a direct `smallvec` dependency and switched `WireFrame` command storage to `SmallVec<[WireCommand; 4]>` (alias `WireCommandBuffer`) so default-size harness batches avoid heap allocating the command list.
- Used `SmallVec` for encode scratch strings, whitespace token splits, and `BATCH` segment splits before parsing.
- Updated the Tokio client to build frames with `WireCommandBuffer`.

What worked:

- Protocol encoding/decoding semantics stay the same; larger batches still spill to the heap transparently.
- `make ci` and `make quality-gate` passed locally before PR.

Measured subset (`throughput_sharded_mixed` / `inmemory`, sample-size 10, short window on this machine):

- Throughput interval about **6.06M to 9.15M elements/s** (Criterion printed `[6.0586, 7.4448, 9.1479] Melem/s`).

What did not work:

- Do not treat this as a guaranteed throughput win until repeated runs show a stable delta versus the prior commit.

Tradeoffs:

- One more direct dependency and slightly more type surface (`WireCommandBuffer`) for a targeted allocation reduction on small frames.

---

## Tokio harness binaries

This repo includes a lightweight benchmark harness server and client:

- `cargo run --bin server -- --bind 127.0.0.1:7001 --instruments 4`
- `cargo run --bin client -- --addr 127.0.0.1:7001 --connections 4 --instruments 4 --batch-size 4 --duration-secs 8`

Protocol is intentionally compact for measurement (not production API):

- `ADD <id> <participant> <instrument> <B|S> <price> <qty>`
- `MARKET <id> <participant> <instrument> <B|S> <qty>`
- `CANCELID <order_id> <instrument>`
- `BATCH <cmd1>|<cmd2>|...` (optional multi-command frame)

Reference run on the same machine:

- `connections=4 duration_s=8 ok_ops=761874 err_ops=0 throughput_ops_s=95234.25 avg_latency_ns=41537.87`

---

## Profiling (engine hot path)

Recommended profile workflow for cleaner hotspot attribution:

```bash
export RUSTFLAGS="-C force-frame-pointers=yes"
export CARGO_PROFILE_BENCH_DEBUG=true
taskset -c 2 cargo flamegraph -p omer --bench throughput_sharded_mixed --features harness,parallel --freq 997 --output docs/perf/flamegraph-throughput_sharded_mixed-inmemory-clean.svg -- --bench --noplot --sample-size 10 --warm-up-time 0.2 --measurement-time 0.5 inmemory
```

If you want kernel frames to resolve more completely on Linux, run:

```bash
sudo sysctl -w kernel.perf_event_paranoid=1
sudo sysctl -w kernel.kptr_restrict=0
```

Then re-run the flamegraph command. For persistence across reboots, add these keys in `/etc/sysctl.d/*.conf`.

For the sharded mixed path, a sample flamegraph artifact is included at:

- [`docs/perf/flamegraph-throughput_sharded_mixed.svg`](docs/perf/flamegraph-throughput_sharded_mixed.svg)
- [`docs/perf/flamegraph-throughput_sharded_mixed-inmemory-clean.svg`](docs/perf/flamegraph-throughput_sharded_mixed-inmemory-clean.svg)

Alternatives: `perf record` + flame graph tooling (see earlier commits or team runbooks). Attach machine type, `rustc -V`, and git SHA with any numbers.

### Candidate improvements after Priority 5

Likely high-impact areas to reduce complexity and improve speed:

- Replace per-level linear queue search in the remaining remove paths with stable location indexing where possible.
- Evaluate narrower data in hot structs (for example, smaller integer types where safe) to improve cache density.
- Prototype `SmallVec` for very short command batches or temporary vectors that are frequently tiny.
- Compare `tokio::mpsc` versus `crossbeam-channel` only on paths that are not forced to be async (sync dispatch can be cheaper).
- Remove repeated work in routing and parsing by reusing parsed command buffers and reducing map churn.

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
