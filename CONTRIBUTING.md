# Contributing

## Engineering process

1. **Issue** — describe the behavior, constraints, and acceptance checks.
2. **Branch** — `feat/…`, `fix/…`, `docs/…`, aligned with the issue.
3. **Incremental commits** — small, reviewable steps; [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, `test:`, `chore:`).
4. **Quality gates** — locally match CI: `make ci`, and `make quality-gate` if `pmat` is installed (see `Makefile` for the default check set; extend with `--include-provability` or `--checks dead-code,entropy` when tightening ITCH/harness coverage).
5. **Pull request** — link the issue, summarize behavior change, note benches or perf impact.
6. **Review & verification** — CI green, coverage budget respected; maintainer merge after sign-off.

## Optional / extended verification

- **Proptest** — `cargo test -p omer proptest_store_roundtrip` (see `tests/proptest_store_roundtrip.rs`).
- **Mutation testing** — `cargo install cargo-mutants` then `cargo mutants -- --all-features` (slow; run before risky refactors).
- **Fuzzing** — add `cargo-fuzz` targets when parsing or binary protocols change materially.
- **Kani** — for small pure functions, consider [Kani](https://model-checking.github.io/kani/) (`cargo install kani-verifier && cargo kani -p omer`); not required for every PR.

## Code style

- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- No `unsafe` without explicit `// SAFETY:` and review (crate default is `forbid`).
