SHELL := /bin/bash

CARGO ?= cargo
MAKE ?= make

.PHONY: check fmt fmt-check clippy test deny doc machete ci cov-info quality-gate

check:
	$(CARGO) check --all-features --all-targets

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --all-features --all-targets -- -D warnings

test:
	$(CARGO) test --workspace --all-features --all-targets

doc:
	$(CARGO) doc --all-features --no-deps

cov-info:
	$(CARGO) llvm-cov --workspace --all-features --all-targets --summary-only

deny:
	$(CARGO) deny check

machete:
	@if ! $(CARGO) --list | grep -q 'machete'; then \
		echo "Installing cargo-machete..."; \
		$(CARGO) install cargo-machete --locked; \
	fi
	$(CARGO) machete --with-metadata

ci:
	$(MAKE) fmt-check
	$(MAKE) clippy
	$(MAKE) test
	$(MAKE) doc
	$(MAKE) deny
	$(MAKE) machete

# Optional: install `pmat` (`cargo install pmat`). Default checks avoid noisy per-file dead-code
# and entropy heuristics on generated-style ITCH helpers; extend locally as needed.
PMAT_CHECKS ?= complexity,satd,security,duplicates,sections
quality-gate:
	@if command -v pmat >/dev/null 2>&1; then \
		pmat quality-gate --project-path . --fail-on-violation --checks $(PMAT_CHECKS); \
	else \
		echo "pmat not installed; skipping (cargo install pmat)"; \
		exit 0; \
	fi