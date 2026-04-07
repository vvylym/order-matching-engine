# Performance artifacts

This folder stores generated profiling artifacts used in README examples.

- `flamegraph-throughput_sharded_mixed.svg`: flamegraph captured from the `throughput_sharded_mixed` Criterion bench.

Notes for reproducibility:

- Build/profile command is documented in the top-level `README.md`.
- On Linux, `perf` permissions (`perf_event_paranoid`, `kptr_restrict`) can affect symbol resolution.
- If samples are dropped under load, re-run with fewer background tasks or a longer measurement window.
