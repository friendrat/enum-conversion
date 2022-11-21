cargo := $(env) cargo
nightly := nightly-2022-05-20


build:
	$(cargo) build

clippy:
	$(cargo) +$(nightly) clippy --all-targets -- -D warnings

check:
	$(cargo) check

fmt:
	$(cargo) +$(nightly) fmt --all

test:
	cd enum-conversion-derive && \
	$(cargo) test && \
	cd .. && \
	$(cargo) test

expand:
	cargo +$(nightly) expand