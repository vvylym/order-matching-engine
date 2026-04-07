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

Tokio harness (mixed load):

```bash
cargo run --bin server -- --bind 127.0.0.1:7001 --instruments 4 --worker-channel tokio --price-book btree
cargo run --bin client -- --addr 127.0.0.1:7001 --connections 4 --instruments 4 --batch-size 4 --duration-secs 8
```

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
- **Harness `client`:** generate mixed workloads; print op and latency counters.

## Benchmarks

**How results here were measured**

- End-to-end **Tokio harness** (not in-process Criterion only).
- **4** concurrent clients (`--connections 4`).
- Fixed duration **8** seconds per run (`--duration-secs 8`).
- Mixed workload shape: add / cancel-by-id / market with **`--batch-size 4`** on the client.
- Server: **`--worker-channel tokio`**, **`--instruments 4`**.
- Table uses **total operations tested** = `ok_ops + err_ops` from the client line (includes protocol rejections in the harness path).

**Top 3 engine settings** (PriceBook × OrderStore × EventSink), same harness flags otherwise:

| PriceBook | OrderStore | EventSink | Clients | Ops tested (`ok_ops + err_ops`) | Total time |
| --- | --- | --- | ---: | ---: | ---: |
| `DashSkipOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 2,041,836 | 8s |
| `PoolLevelOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 2,019,472 | 8s |
| `BTreeOrderBook` | `HashMapOrderStore` | `NoOpEventSink` | 4 | 1,957,996 | 8s |

## Contributing

Workflow: issue → branch → small commits → PR → review. Run `make ci` before pushing; optionally `make quality-gate`. Details: [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Licenses

Project code is released under the [MIT License](LICENSE).
