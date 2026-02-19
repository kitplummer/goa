.PHONY: build test test-all lint coverage clean release help

help:
	@echo "Available targets:"
	@echo "  build     - Build the project in debug mode"
	@echo "  release   - Build the project in release mode"
	@echo "  test      - Run fast tests (no network)"
	@echo "  test-all  - Run all tests including network-dependent ones"
	@echo "  lint      - Run clippy linter"
	@echo "  coverage  - Generate test coverage report (requires cargo-llvm-cov)"
	@echo "  clean     - Clean build artifacts"

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

test-all:
	cargo test -- --include-ignored

lint:
	cargo clippy -- -D warnings

coverage:
	cargo llvm-cov --html
	@echo "Coverage report generated at target/llvm-cov/html/index.html"

clean:
	cargo clean
