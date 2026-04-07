# omer

[![CI](https://github.com/vvylym/order-matching-engine/actions/workflows/ci.yml/badge.svg)](https://github.com/vvylym/order-matching-engine/actions/workflows/ci.yml)

Rust order matching core: limit/market matching on a price–time book. Commands flow through [`OrderMatchingService`](https://docs.rs/omer/latest/omer/engine/trait.OrderMatchingService.html). The repo also ships Tokio **`server`** / **`client`** binaries for local harness measurements.

**Repository:** [github.com/vvylym/order-matching-engine](https://github.com/vvylym/order-matching-engine) · **crate:** `omer`

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
omer = "0.1"
```

Optional feature **`parallel`** (pulls in `rayon`):

```toml
omer = { version = "0.1", features = ["parallel"] }
```

Library-only (no default harness):

```toml
omer = { version = "0.1", default-features = false }
```

Optional **`rkyv`** binary wire helpers for the harness protocol (experimental; see `distributed_wire` when built with `--features rkyv`):

```toml
omer = { version = "0.1", features = ["rkyv"] }
```

## Usage

Minimal engine call:

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

Composition via `omer::engine::builder()` is documented in crate rustdoc and [`CONTRIBUTING.md`](CONTRIBUTING.md).

Tokio harness — **duration stop** (wall time bound):

```bash
cargo run --release --bin server -- --bind 127.0.0.1:7001 --instruments 32 --worker-channel tokio --price-book btree
cargo run --release --bin client -- --addr 127.0.0.1:7001 --connections 4 --instruments 32 --batch-size 8 --random --duration-secs 60
```

Tokio harness — **operation-count stop** (reports **measured** `wall_time_s` until `ok_ops` ≥ target; no fixed duration):

```bash
cargo run --release --bin server -- --bind 127.0.0.1:7001 --instruments 32 --worker-channel tokio --price-book dash_skip
cargo run --release --bin client -- \
  --addr 127.0.0.1:7001 --connections 4 --instruments 32 --batch-size 8 \
  --random --seed 42 --target-ok-ops 10000000
```

**Harness `server` matcher type:** workers use a concrete `enum MatcherEngine { Btree(..), DashSkip(..), PoolLevel(..) }` that forwards to [`OrderMatchingEngine`](src/engine/service.rs). There is **no** `Box<dyn OrderMatchingService>` on the matcher hot path. **`Arc` is not a substitute** for the matcher value here: each instrument worker must hold **exclusive** `&mut` access to its own engine and mutate book/store state; `Arc` shares **immutable** ownership and would still require interior mutability (`Mutex`/`RwLock`) and contention on every command.

Local quality gates:

```bash
make ci
make quality-gate
```

## Repository layout

| Path | Role |
|------|------|
| `src/lib.rs` | Crate root |
| `src/types.rs` | Order ids, side, TIF, numeric aliases |
| `src/engine/` | `OrderMatchingService`, commands, builder |
| `src/book/`, `src/store/`, `src/events/` | Book, store, event sink traits and impls |
| `src/distributed_wire.rs` | Shared harness wire types and text codec |
| `src/bin/server.rs`, `src/bin/client.rs` | Tokio benchmark harness |
| `benches/` | Criterion benches |
| `tests/` | Integration and property tests |
| `docs/` | Design and perf artifacts |
| `scripts/` | Coverage and helpers |

## Services

- **Matching:** [`OrderMatchingService`](src/engine/mod.rs) — process `OrderCommand` values (`Add`, `Cancel`, `Replace`, `CancelByOrderId`, etc.).
- **Harness `server`:** parse wire frames, route by instrument, dispatch to workers.
- **Harness `client`:** generate workloads (`--random` = stochastic mix of limits / cancels / markets across instruments); print op and latency counters.

## Benchmarks

### How these numbers were produced (no interpolation)

Each table row is **one real end-to-end run** on this host:

- **Build:** `cargo build --release --bin server --bin client`
- **Stop rule:** `--target-ok-ops N` — clients exit once aggregate **`ok_ops` ≥ N** (tiny overshoot is possible because batches are applied atomically per frame).
- **Measured quantity:** client line `wall_time_s=…` is **actual elapsed wall time** for that run (not `N / throughput`).
- **Load:** `--connections 4`, `--instruments 32`, `--batch-size 8`, **`--random`**, **`--seed 42`** (reproducible RNG; mix is not a fixed mod-10 pattern).
- **Server:** `--worker-channel tokio`; `--instruments 32` matches the client’s routing range.
- **Store / sink:** `HashMapOrderStore`, `NoOpEventSink` (server defaults for the harness).

**500M rows:** a full 500 M–command run was **not** retained in this documentation refresh: a long `dash_skip` attempt drove very large resident memory on this machine. Re-run locally when you have headroom, using the same command pattern as below and your chosen `--price-book` / `--instruments`.

```bash
cargo run --release --bin server -- --bind 127.0.0.1:7001 --instruments 32 --price-book dash_skip --worker-channel tokio
cargo run --release --bin client -- --addr 127.0.0.1:7001 --connections 4 --instruments 32 --batch-size 8 --random --seed 42 --target-ok-ops 500000000
```

### Measured throughput ladder (target `ok_ops` vs wall time)

| Target `ok_ops` (stop) | `ok_ops` (actual) | Wall time (s) | PriceBook | OrderStore | EventSink | Clients | Instruments | Random |
| ---: | ---: | ---: | --- | --- | --- | ---: | ---: | --- |
| 1,000,000 | 1,000,024 | 0.851107 | `DashSkipOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 32 | yes |
| 10,000,000 | 10,000,024 | 10.046481 | `DashSkipOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 32 | yes |
| 50,000,000 | 50,000,024 | 94.083841 | `DashSkipOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 32 | yes |
| 100,000,000 | 100,000,024 | 327.418597 | `DashSkipOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 32 | yes |
| 1,000,000 | 1,000,024 | 1.046007 | `PoolLevelOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 32 | yes |
| 10,000,000 | 10,000,024 | 11.986405 | `PoolLevelOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 32 | yes |
| 50,000,000 | 50,000,024 | 100.141284 | `PoolLevelOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 32 | yes |
| 100,000,000 | 100,000,024 | 334.108022 | `PoolLevelOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 32 | yes |
| 1,000,000 | 1,000,024 | 0.907989 | `BTreeOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 32 | yes |
| 10,000,000 | 10,000,024 | 11.853820 | `BTreeOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 32 | yes |
| 50,000,000 | 50,000,024 | 105.312649 | `BTreeOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 32 | yes |
| 100,000,000 | 100,000,024 | 344.068611 | `BTreeOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 32 | yes |

## Contributing

Workflow: issue → branch → small commits → PR → review. Run `make ci` before pushing; optionally `make quality-gate`. Details: [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Licenses

Project code is released under the [MIT License](LICENSE).
