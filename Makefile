SHELL := /bin/bash

CARGO ?= cargo
MAKE ?= make

.PHONY: check fmt fmt-check clippy test deny doc machete ci cov-info

check:
	$(CARGO) check --all-features --all-targets

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --all-features --all-targets -- -D warnings

test:
	$(CARGO) test --all-features --all-targets

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