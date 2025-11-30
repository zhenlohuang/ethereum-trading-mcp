.PHONY: help build test fmt clippy coverage coverage-html clean

# Use rustup-managed cargo for coverage (required for llvm-tools-preview)
RUSTUP_CARGO := $(HOME)/.cargo/bin/cargo

help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

build: ## Build the project
	cargo build

build-release: ## Build the project in release mode
	cargo build --release

test: ## Run tests
	cargo test

fmt: ## Format code
	cargo fmt

fmt-check: ## Check code formatting
	cargo fmt --all -- --check

clippy: ## Run clippy linter
	cargo clippy --all-targets --all-features -- -D warnings

coverage: ## Generate test coverage report (text output)
	$(RUSTUP_CARGO) llvm-cov --all-features --workspace

coverage-html: ## Generate HTML test coverage report
	$(RUSTUP_CARGO) llvm-cov --all-features --workspace --html
	@echo "Coverage report generated at: target/llvm-cov/html/index.html"

coverage-lcov: ## Generate LCOV coverage report
	$(RUSTUP_CARGO) llvm-cov --all-features --workspace --lcov --output-path lcov.info
	@echo "LCOV report generated at: lcov.info"

coverage-json: ## Generate JSON coverage report
	$(RUSTUP_CARGO) llvm-cov --all-features --workspace --json --output-path coverage.json
	@echo "JSON report generated at: coverage.json"

clean: ## Clean build artifacts
	cargo clean

ci: fmt-check clippy test ## Run all CI checks locally
