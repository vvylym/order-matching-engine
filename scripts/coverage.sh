#!/usr/bin/env bash
# Line coverage (cargo-llvm-cov). Enforces >= 85% on instrumented lines (engine + ITCH + services).
set -euo pipefail

exec cargo llvm-cov --all-features --all-targets \
  --summary-only \
  --fail-under-lines 85
