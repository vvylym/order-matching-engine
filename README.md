# order-matching-engine (`omer`)

Minimal **order-matching engine** building blocks in Rust: orders, books, matching and self-trade **traits**, commands, and unified errors. This repo is a **spec / library boundary** you can implement behind your own exchange stack.

## Status

The published API is intentionally trait-heavy: concrete matching engines, books, and stores are **your** implementations. Unit tests cover dispatch on `OrderMatchingService`, sequence defaults, error formatting, and type smoke checks.

## Quick start

```bash
cargo test --all-features --all-targets
cargo clippy --all-features --all-targets -- -D warnings
```

Coverage (requires [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)):

```bash
chmod +x scripts/coverage.sh   # once
./scripts/coverage.sh
```

CI enforces **≥ 85% line coverage** on the instrumented lines LLVM reports for this crate (mostly small—most of the codebase is types and trait definitions).

## Crates.io name

The package name is **`omer`** (Rust crate name).

## License

MIT — see [LICENSE](LICENSE).
