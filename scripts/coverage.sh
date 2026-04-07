#!/usr/bin/env bash
# Line coverage (cargo-llvm-cov). Enforces >= 85% on instrumented lines (engine + ITCH + services).
# Tokio harness binaries under src/bin are excluded: they are integration-style entrypoints and are
# not exercised by the unit/integration suite in a way that llvm-cov attributes to those files.
set -euo pipefail

exec cargo llvm-cov --all-features --all-targets \
  --summary-only \
  --fail-under-lines 85 \
  --ignore-filename-regex 'src/bin/(server|client)\.rs'
