.PHONY: build test lint coverage clean release help

help:
	@echo "Available targets:"
	@echo "  build     - Build the project in debug mode"
	@echo "  release   - Build the project in release mode"
	@echo "  test      - Run all tests"
	@echo "  lint      - Run clippy linter"
	@echo "  coverage  - Generate test coverage report (requires cargo-llvm-cov)"
	@echo "  clean     - Clean build artifacts"

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

lint:
	cargo clippy -- -D warnings

coverage:
	cargo llvm-cov --html
	@echo "Coverage report generated at target/llvm-cov/html/index.html"

clean:
	cargo clean
