.PHONY: build lint fmt clean help

help:
	@echo "Available commands:"
	@echo "  make build      - Build the project"
	@echo "  make lint       - Run clippy linter"
	@echo "  make fmt        - Format code"
	@echo "  make test       - Run tests"
	@echo "  make clean      - Clean build artifacts"
	@echo "  make validate   - Format, lint, and build"

build:
	cargo build

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

test:
	cargo test

clean:
	cargo clean

validate: fmt lint test build
