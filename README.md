# order-matching-engine (`omer`)

Single-crate **order matching engine** in Rust: a concrete `OrderMatchingEngine` generic over book, store, sequence, policies, and event sink; **ITCH** ingest helpers; **Criterion** benches; and a broad **integration test matrix** (limits, markets, cancel/replace, determinism, stress).

Public surface follows the **`OrderMatchingService`** port: `add`, `cancel`, `replace`, **`cancel_by_order_id`**, **`reduce`**, **`execute`**, **`replace_by_new_id`**, and `process` over `OrderCommand` (see [`engine/models.rs`](src/engine/models.rs)).

## Layout (single crate)

| Module | Role |
|--------|------|
| `engine` | Commands, `OrderMatchingService` trait, `OrderMatchingEngine` implementation |
| `book` / `store` | `PriceBook` / `OrderStore` traits + in-crate `btree`, `pool_level`, `dense`, `hash_map` services |
| `matching` / `self_trade` / `sequence` / `events` | Policies and event sink trait |
| `itch` | Buffered NASDAQ ITCH-style feed parsing wired into `OrderMatchingEngine` |

## Quick start

```bash
cargo test --all-features --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo bench   # Criterion (local); CI compiles benches with `cargo bench --no-run`
```

Planned bench categories and implementation order: [`benches/PLAN.md`](benches/PLAN.md).

Coverage (requires [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)):

```bash
chmod +x scripts/coverage.sh   # once
./scripts/coverage.sh
```

CI enforces **≥ 85% line coverage** on instrumented lines for this crate (`cargo llvm-cov` summary).

## Tests

- **`tests/`** — semantics, integrity, replay, observability, self-trade, property tests (`rstest`, `quickcheck`), ITCH fixture check.
- **`tests/market_manager.rs`** — ignored stubs for future **MarketManager** / CppTrader-style scenarios; see [`benches/PLAN.md`](benches/PLAN.md) for the intended bench matrix (not run in PR CI).
- **`tests/matching_engine.rs`** — one ignored integration benchmark-style test pending tightened stat assertions.

## Consolidation note

Implementation and ITCH stack derive from the local **`Work/omer`** tree; integration tests from **`Work/order-matching-engine`**, rebased on the extended command API (`symbol_id` and optional fields on `AddOrderCommand`, etc.). A future **workspace split** (lib + thin gateway binaries) is described in the repo planning doc but is **not** required for this single-crate snapshot.

## Performance and safety policy

- **`unsafe`:** The crate keeps **`unsafe_code = forbid`** unless a change introduces a **minimal** `unsafe` scope with **`// SAFETY:`** comments and review—never by default for speed.
- **CI vs load tests:** Default **PR CI** runs correctness gates (fmt, clippy, tests, coverage, audit, `cargo bench --no-run`). It does **not** run gateway daemons, long soak tests, or full benchmark **executions**. When gateway tests, load tests, or full benches are run elsewhere, **document results** (command, hardware, git SHA, numbers) in the PR or a perf log before merging changes that claim throughput milestones.
- **Throughput metric:** Any **>1B orders/sec** headline means **end-to-end** throughput—orders that complete the full **client → gateway → matcher** path (validated, routed, and applied by the matcher under an explicit counting rule)—**not** gateway ingress or accept-only counts.

## Performance roadmap

Correctness and coverage first; distributed **gateway + TCP** and **>1B orders/sec (E2E)** remain future Phase 5 work; see project planning docs for scope.

## Crates.io name

The package name is **`omer`** (Rust crate name).

## License

MIT — see [LICENSE](LICENSE).
