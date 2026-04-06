#!/usr/bin/env bash
# Line coverage (cargo-llvm-cov). Enforces >= 85% on instrumented lines.
# This crate is mostly type and trait definitions; executable surface is small but gated here.
set -euo pipefail

exec cargo llvm-cov --all-features --all-targets \
  --summary-only \
  --fail-under-lines 85
