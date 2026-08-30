.PHONY: build test check fmt clean

build:
	cargo build --target wasm32v1-none --release --locked
	mkdir -p target/wasm32v1-none/optimized
	stellar contract optimize \
		--wasm target/wasm32v1-none/release/blnt_backfill_contract.wasm \
		--wasm-out target/wasm32v1-none/optimized/blnt_backfill_contract.wasm

test:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features --locked -- -D warnings
	cargo test --all-features --locked

check:
	cargo check --all-targets --all-features --locked

fmt:
	cargo fmt --all

clean:
	cargo clean
