# Performance artifacts

This folder stores generated profiling artifacts used in README examples.

- `flamegraph-throughput_sharded_mixed.svg`: earlier sharded mixed flamegraph sample.
- `flamegraph-throughput_sharded_mixed-inmemory-clean.svg`: pinned-core run focused on the `inmemory` benchmark function.

Notes for reproducibility:

- Build/profile command is documented in the top-level `README.md`.
- Prefer CPU pinning for cleaner call stacks:
  - `taskset -c 2` (or any mostly idle core on your machine).
- Keep frame pointers and bench debug symbols enabled:
  - `RUSTFLAGS="-C force-frame-pointers=yes"`
  - `CARGO_PROFILE_BENCH_DEBUG=true`
- Limit Criterion output/tooling noise:
  - use `--noplot`,
  - filter to one bench function when investigating one hotspot.
- On Linux, `perf` permissions (`perf_event_paranoid`, `kptr_restrict`) can affect symbol resolution.
- If samples are dropped under load, re-run with fewer background tasks or a longer measurement window.
