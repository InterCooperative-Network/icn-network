.PHONY: all test unit-tests integration-tests bench build docs clean metrics reputation priority relay integrated demo

all: build

# Building
build:
	cargo build

# Running tests
test: unit-tests integration-tests

unit-tests:
	cargo test

integration-tests:
	./scripts/test_all_features.sh

bench:
	cargo bench

# Generate documentation
docs:
	cargo doc --open

# Run individual demos
metrics:
	cargo run --example metrics_demo

reputation:
	cargo run --example reputation_demo

priority:
	cargo run --example priority_messaging

relay:
	cargo run --example circuit_relay_demo relay-server

integrated:
	cargo run --example integrated_demo

# Run all demos via the script
demo:
	./scripts/run_demos.sh

# Cleanup
clean:
	cargo clean 