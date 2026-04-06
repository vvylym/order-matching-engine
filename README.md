# omer — order matching engine

[![CI](https://github.com/vvylym/order-matching-engine/actions/workflows/ci.yml/badge.svg)](https://github.com/vvylym/order-matching-engine/actions/workflows/ci.yml)

Rust library that matches **limit and market orders** against a **price–time** book: add, cancel, replace, and related commands go through one **`OrderMatchingEngine`** type. Numbers are integers (ticks / lots); there is no `f64` on the hot path.

**Repository:** [github.com/vvylym/order-matching-engine](https://github.com/vvylym/order-matching-engine)  
**Crate name on crates.io:** `omer` (same as the package in `Cargo.toml`).

If **GitHub’s README looks stale**, compare the **commit on `main`** to your local tree — the default branch only changes after a **push** (or merged PR).

---

## Performance roadmap (matching hot path → 1B+ ops/s in-process)

Roadmap for **CPU-bound** add/cancel/match work (no sockets). **1B+ ops/s** here means the **matcher + book** sustaining that aggregate rate on suitable hardware (many cores, batching, sharding — see Phase 4), not a single `localhost` TCP loop.

### Phase 1 (week 1–2) — book structure + first throughput band

- [x] Add **`dashmap`** dependency  
- [x] Per-side book: **`DashMap<Price, Vec<Order>>`** for level queues + **`SkipMap<Price, ()>`** for best bid / best ask  
- [x] **`DashSkipOrderBook`** in [`src/book/service/dash_skip.rs`](src/book/service/dash_skip.rs) (implements [`PriceBook`](src/book/mod.rs))  
- [x] Benchmark **`throughput_book`** — target **~200–300M pushes/sec** on strong CPUs (run locally; record CPU + `rustc -V`)  
- [x] **`latency_add`** compares **InMemory**, **BTree**, **PoolLevel**, **DashSkip** behind the same harness ([`benches/latency_add.rs`](benches/latency_add.rs))  

### Phase 2 (week 2–3) — pooling + parallel batches

- [ ] **`OrderPool`** / arena for resting `Order` payloads (reduce `clone` pressure on book levels)  
- [x] **Batch API:** [`OrderMatchingEngine::process_batch`](src/engine/service.rs) (`OrderCommand` iterator; commits in order, stops on first error)  
- [ ] **`rayon`** (optional feature) for **sharded** or read-mostly paths — single-writer book stays serial  
- [x] Engine throughput bench **[`throughput_engine`](benches/throughput_engine.rs)** — warm book + **`process_batch`** of 512 resting adds, **four** `PriceBook` backends, **`NoOpEventSink`**; local target band **~300–500M adds/sec** class on big cores (measure; not CI-gated)  

### Phase 3 (week 3–4) — order-type specialization

- [x] Rich **`Order`** / TIF types (already in crate)  
- [ ] **`OrderBehavior`**, trait-based or enum-dispatched fast paths per order type  
- [ ] Benches **per order type**; **no regression** vs baseline commits  

### Phase 4 (week 4–5) — sharding + micro-architecture

- [ ] **`ShardedOrderBook`** (partition by price / symbol)  
- [ ] **SIMD** helpers where profiling proves they win  
- [ ] **CPU affinity** / pinning notes or cfg for deployment  
- [ ] Benchmark band **~500M–1B+ ops/sec** aggregate in-process; **perf / flamegraph** in [`benches/PLAN.md`](benches/PLAN.md)  

---

## What you get today

| Area | Status |
|------|--------|
| **Matching** | Single-symbol engine: limit/market, partial fills, IOC/GTC and related TIF, replace, cancel-by-id, reduce, execute (see `OrderCommand` in [`src/engine/models.rs`](src/engine/models.rs)). |
| **Storage / book** | **`PriceBook`** + **`OrderStore`**: `BTreeOrderBook`, **`DashSkipOrderBook`** (DashMap + SkipMap, Phase 1), `PoolLevelOrderBook`, `dense` / `hash_map` stores. |
| **ITCH** | NASDAQ ITCH-style **binary messages** can be read from a stream and turned into engine commands (`src/itch/`). |
| **Tests** | Integration tests under [`tests/`](tests/) (semantics, replay, self-trade, stress/property checks). CI runs them on every push/PR. |
| **Coverage** | CI fails if **line** coverage (llvm-cov) falls **below 85%** — see [`scripts/coverage.sh`](scripts/coverage.sh). |

---

## Requirements

- **Rust:** stable toolchain (same as CI; edition 2024 in `Cargo.toml`).
- **Optional:** [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) locally to match the coverage job.

---

## Quick start

Clone the repo, then from the crate root:

```bash
cargo test --all-features --all-targets
```

Run the linter the same way CI does:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Line coverage (after installing `cargo-llvm-cov`):

```bash
chmod +x scripts/coverage.sh
./scripts/coverage.sh
```

---

## How the crate is organized

Read top-down:

1. **[`src/lib.rs`](src/lib.rs)** — short overview of modules (crate docs).
2. **[`src/engine/`](src/engine/)** — **`OrderMatchingService`** trait and **`OrderMatchingEngine`** implementation.
3. **[`src/types.rs`](src/types.rs)** — **`Order`**, prices, quantities, sides, TIF.
4. **`src/book/`, `src/store/`** — traits plus concrete services.
5. **`src/itch/`** — wire layout + streaming entry points.
6. **`src/harness/`** (feature **`harness`**, **on by default**) — shared store, policies, and event sink; pick the book with **`engine_with_book`** / **`engine_with_*`**. Use **`engine_with_book_noop`** when benchmarks must not allocate on **`Event::Accepted`**. Omit harness: `omer = { version = "…", default-features = false }`.

---

## Benchmarks and performance (today)

| Bench | What it does right now |
|-------|-------------------------|
| **`latency_add`** | Same harness, **four `PriceBook` backends** (in-memory B-tree levels, `BTreeOrderBook`, `PoolLevelOrderBook`, `DashSkipOrderBook`): one resting buy `add` per iter. Run: `cargo bench -p omer --bench latency_add`. |
| **`throughput_book`** | **`PriceBook::push`** on **`DashSkipOrderBook`**: same-price FIFO vs distinct-price levels (50k ops/iter). Run: `cargo bench -p omer --bench throughput_book`. |
| **`throughput_engine`** | Warmed engine + **`process_batch`** (512 resting limit buys), **four** book backends, **`NoOpEventSink`**. Run: `cargo bench -p omer --bench throughput_engine`. |
| **`itch_parse`** | **`scan_decode_book_messages`** on a buffer of **AddOrder** packets (decode only). Run: `cargo bench -p omer --bench itch_parse`. |
| **`micro`**, **`market_manager`**, **`matching_engine`** | **Placeholder** loops so `cargo bench --no-run` keeps targets building; see [`benches/PLAN.md`](benches/PLAN.md). |

**North star:** **10⁹+ in-process operations per second** on the **matcher + book** path is a **program target** (sharding, batching, SIMD, cores — Phase 4), not something a single-threaded integration test proves. Network I/O is out of scope for that number: measure **decode**, **book/engine**, and (later) **gateway** separately. Always record hardware and `rustc` with published figures.

**Pull request CI** compiles all benches but **does not** run long or distributed load tests (keeps feedback fast). If you run heavy benchmarks locally, record machine, git revision, and command in the PR text when you report numbers.

**Safety:** `unsafe` is **forbidden** at crate level unless a future change adds a **small**, reviewed block with an explicit `// SAFETY:` note.

---

## Contributing

Issues and PRs are welcome. Match **existing style** (formatting, `clippy -D warnings`, tests). Use **conventional commits** if you can (`feat:`, `fix:`, `docs:`, …). For larger changes, open an issue first so direction matches maintainers’ expectations.

---

## License

[MIT](LICENSE).
