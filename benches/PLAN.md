# Benchmark plan: per-category benches

**Note:** Numbered README sections (§1–§7) cited below are **aspirational targets** from the upstream bench README this file was ported with. This repository’s [README.md](../README.md) may not duplicate every section; use this document as the **roadmap** for future Criterion targets and implementation order.

This plan maps each category to individual benchmark files under `benches/`, with targets, metrics, and implementation notes.

### Scenario backlog (CppTrader parity)

Topics for a future **MarketManager** (or multi-book façade): **market order**, **limit** crossing, **IOC**, **FOK**, **AON**, **iceberg** / hidden quantity, **stop** and **stop-limit**, **trailing** stop, manual matching, in-flight mitigation. Implementation lives in the engine/harness first; orchestration benches come after.

---

## Prerequisites (shared setup)

- **Cargo**: Add `[[bench]]` entries for each bench and a dev-dependency on `criterion` (e.g. `criterion = { version = "0.5", features = ["html_reports"] }`).
- **Engine in benches**: Bench binaries need a concrete engine. Options:
  - **A)** Crate **`harness`** feature (default-on): exports [`omer::harness`](../src/harness/mod.rs) with `InMemoryPriceBook`, `InMemoryOrderStore`, `CollectingEventSink`, `IncrementalSequence`, policies, and `engine_with_memory()` for benches and integration tests.
  - **B)** Duplicate wiring only inside a bench-local `benches/common.rs` (avoid if **A** exists).
- **Recommendation**: Use **`harness`** for one source of truth; use a no-op or minimal event sink in latency benches if collection alloc skews numbers.

---

## 1. Correctness & determinism → `correctness.rs`

**README:** §1 — strict price–time priority, deterministic matching, no float in price/qty, exact replay; 0 divergent replays over ≥10⁶ runs, bit-identical state, 100% invariants.

**Role of this bench:** Correctness is primarily enforced by **tests** (e.g. replay tests, invariant checks). The bench’s job is to ensure that **replay and state checks are cheap enough** to run at scale (e.g. 10⁶ replays in reasonable time), and to **detect regressions** in the cost of these checks.

**Bench design:**

- **Name:** `correctness` (e.g. `benches/correctness.rs`).
- **What to measure:**
  - Time to run **N full replays** (e.g. N = 10⁴ or 10⁵ per iteration) from a fixed event log or script, with a **state checksum** (or snapshot comparison) after each replay.
  - Optionally: time to **build one deterministic snapshot** from the same input stream.
- **Target:** No hard latency target here; the goal is to keep “replay + checksum” fast enough that 10⁶ runs in CI is feasible (e.g. report “replays per second” or “time per 10⁵ replays”).
- **Implementation notes:**
  - Reuse or mirror the same replay path used in tests (event log or commands → engine → final state).
  - Use a deterministic workload (fixed seed or fixed script); checksum book + store (or equivalent) after each run.
  - Can use a no-op or count-only event sink to focus on matching + state update cost.

**Cargo:** `[[bench]] name = "correctness" harness = false`

---

## 2. Latency (single-threaded, hot path) → `latency_add.rs`, `latency_cancel.rs`, `latency_replace.rs`, `latency_market.rs`

**README:** §2 — Add/Cancel/Modify p50 ≤ 300 ns, p99 ≤ 1.5 µs; market order match (single level) p50 ≤ 500 ns, p99 ≤ 2 µs; “pride” sub-200 ns p50, sub-1 µs p99.

**Bench design:**

- **`latency_add`** (**implemented:** `benches/latency_add.rs`, uses `omer::harness`)  
  - **What:** Time per **add** (limit order, resting on book).  
  - **How:** Criterion per-iteration: one `engine.add(add_cmd)`; pre-populate book if needed so the add doesn’t match.  
  - **Report:** p50 / p95 / p99 (e.g. Criterion’s `SamplingMode::Flat` or similar for ns-scale).  
  - **Target:** p50 ≤ 300 ns, p99 ≤ 1.5 µs (respectable); pride: p50 &lt; 200 ns, p99 &lt; 1 µs.

- **`latency_cancel`**  
  - **What:** Time per **cancel** (existing order).  
  - **How:** Each iteration: ensure one order exists then `engine.cancel(cancel_cmd)`.  
  - **Target:** Same as add.

- **`latency_replace`**  
  - **What:** Time per **replace** (cancel-replace).  
  - **How:** Each iteration: ensure order exists then `engine.replace(replace_cmd)`.  
  - **Target:** Same ballpark as add/cancel (README groups them).

- **`latency_market`**  
  - **What:** Time per **market order match** (single price level).  
  - **How:** Each iteration: ensure one resting level on the book, then submit one market order that matches that level fully (or fixed size).  
  - **Target:** p50 ≤ 500 ns, p99 ≤ 2 µs; pride: tighter.

**Shared:** Use a **no-op or minimal event sink** (no logging, no alloc) and warm cache (e.g. run a few thousand ops before measuring). Single-threaded only.

**Cargo:** One `[[bench]]` per file, e.g. `latency_add`, `latency_cancel`, `latency_replace`, `latency_market`, all with `harness = false`.

---

## 3. Throughput (sustained) → `throughput_mixed.rs`, optional `throughput_adversarial.rs`

**README:** §3 — ≥ 2–5 M ops/sec mixed add/cancel/match (respectable); ≥ 10 M ops/sec pride; no latency cliff under adversarial patterns (e.g. deep book, heavy cancels).

**Bench design:**

- **`throughput_mixed`**
  - **What:** Sustained ops/sec on a **mixed** workload (add / cancel / match) on one core, warm cache, no logging.
  - **How:** Long loop of `process(OrderCommand)` (or add/cancel/replace in proportion); e.g. 60% add, 20% cancel, 20% market (tunable). Measure total time for a fixed number of ops (e.g. 1M–10M), report **ops/sec**.
  - **Target:** ≥ 2–5 M ops/sec (respectable); ≥ 10 M ops/sec (pride).

- **`throughput_adversarial`** (optional)
  - **What:** Same as above but with **adversarial** pattern: deep book (e.g. 50k+ levels), heavy cancel rate, or sweep market orders.
  - **Goal:** Ensure throughput doesn’t collapse (README: “If throughput collapses when the book has 50k+ price levels, your structure is wrong”).
  - **Report:** ops/sec and, if useful, p99 latency over the run.

**Cargo:** `[[bench]] name = "throughput_mixed" harness = false`, and optionally `throughput_adversarial`.

---

## 4. Memory discipline & layout → `memory_hot_path.rs` (and/or alloc count)

**README:** §4 — Zero allocations on hot path, stable footprint, O(1) order lookup, cache-line-aware layout; L1 hit rate &gt; 95%, no allocator calls after warm-up.

**Bench design:**

- **`memory_hot_path`**
  - **What:** Confirm **no allocations** on the hot path after warm-up.
  - **How:** Use a custom allocator (e.g. `#[global_allocator]` with a tracking allocator that counts allocations) or an allocator that aborts on alloc after warm-up; run a large number of add/cancel/match ops and assert allocation count is zero (or only from initial structures).
  - **Alternative:** Integrate with a profiler (e.g. `heaptrack`, `dhat`) and document “zero allocs after warm-up” as a manual or CI check.
  - **Report:** Allocation count per N ops (target: 0 after warm-up), or “pass/fail” in CI.

**Cargo:** `[[bench]] name = "memory_hot_path" harness = false`. May need dev-dependency for a tracking allocator (e.g. `tikv-jemallocator` with profiling or a small custom wrapper).

**Note:** L1 hit rate and cache-line layout are usually observed via perf/profiling rather than a single bench; the plan can reference “see docs” or a separate profiling script.

---

## 5. Market integrity → covered by tests; optional `integrity_stress.rs`

**README:** §5 — Self-trade prevention, atomic cancel-replace, partial fill correctness, consistent top-of-book; fuzzing with random cancels, IOC/FOK, deep sweep; zero invariant violations over ≥10⁷ events.

**Role:** Market integrity is primarily enforced by **tests** (e.g. fuzz, invariant checks). An optional bench can stress the **throughput of integrity checks** or the **cost of running a long integrity-heavy workload**.

- **`integrity_stress`** (optional)
  - **What:** Run a **large** stream of events that stress integrity (random cancels, IOC/FOK, deep sweeps) and optionally **verify invariants** at the end (or at checkpoints).
  - **How:** Either (a) time to run 10⁶–10⁷ such events and optionally assert no invariant violations, or (b) run the same fuzz harness as tests but in a bench to measure “events per second” under integrity-heavy load.
  - **Target:** Zero violations; report events/sec for comparison.

**Cargo:** Optional `[[bench]] name = "integrity_stress" harness = false`.

---

## 6. Observability → optional `observability_overhead.rs`

**README:** §6 — Per-operation latency histogram, queue depth, event lag, deterministic event IDs; ability to reproduce from logs.

**Role:** Observability is mostly design and instrumentation. A small bench can measure the **overhead of emitting events** (or recording latencies) so that we know the cost of “full” observability.

- **`observability_overhead`** (optional)
  - **What:** Compare **latency or throughput** with a **no-op sink** vs with a **real event sink** (e.g. collecting events or writing to a buffer) to quantify overhead of event emission and deterministic IDs.
  - **How:** Same workload in two modes; report ratio or delta (e.g. “sink adds X ns per op” or “Y% throughput drop”).
  - **Target:** Document the overhead; no specific numeric target from README.

**Cargo:** Optional `[[bench]] name = "observability_overhead" harness = false`.

---

## 7. What not to optimize (no dedicated bench)

**README:** §7 — Don’t optimize multi-threading, SIMD, lock-free, or networking before single-thread and memory are in place. No separate bench; the above benches are all single-threaded and in-process.

---

## Summary: bench list and Cargo.toml

| Category              | Bench file(s)              | Main metric / target                          |
|-----------------------|----------------------------|-----------------------------------------------|
| Correctness           | `correctness.rs`           | Replays per second; 10⁶ replays feasible      |
| Latency               | `latency_add.rs`           | p50/p99 ns per add                            |
|                       | `latency_cancel.rs`        | p50/p99 ns per cancel                         |
|                       | `latency_replace.rs`       | p50/p99 ns per replace                        |
|                       | `latency_market.rs`        | p50/p99 ns per market match                   |
| Throughput            | `throughput_mixed.rs`      | ops/sec (2–5M respectable, 10M pride)         |
|                       | `throughput_adversarial.rs`| ops/sec with deep book / heavy cancel         |
| Memory                | `memory_hot_path.rs`       | 0 allocs after warm-up                        |
| Market integrity      | `integrity_stress.rs`      | Optional; events/sec, 0 violations            |
| Observability         | `observability_overhead.rs`| Optional; sink overhead                       |

**Suggested `Cargo.toml` additions:**

```toml
[dev-dependencies]
# ... existing ...
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "correctness"
harness = false

[[bench]]
name = "latency_add"
harness = false

[[bench]]
name = "latency_cancel"
harness = false

[[bench]]
name = "latency_replace"
harness = false

[[bench]]
name = "latency_market"
harness = false

[[bench]]
name = "throughput_mixed"
harness = false

[[bench]]
name = "throughput_adversarial"
harness = false

[[bench]]
name = "memory_hot_path"
harness = false
# Optional:
# [[bench]]
# name = "integrity_stress"
# harness = false
# [[bench]]
# name = "observability_overhead"
# harness = false
```

**Implementation order (suggested):**

1. Add feature `bench` (or equivalent) and shared engine construction so that benches can build `EngineWithMemory` (or equivalent) with a no-op/minimal sink for latency/throughput.
2. Add Criterion and `[[bench]]` entries.
3. Implement **latency_*** (add, cancel, replace, market) — they give immediate, README-aligned targets.
4. Implement **throughput_mixed** (and optionally throughput_adversarial).
5. Implement **correctness** (replay + checksum).
6. Implement **memory_hot_path** (tracking allocator or doc + manual check).
7. Add optional **integrity_stress** and **observability_overhead** if desired.

This keeps the plan aligned with the four non-negotiable classes (correctness, latency, throughput, memory) plus market integrity and observability as in the README.
