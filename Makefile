.PHONY: help dev run build release lint format test clean check

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

dev: ## Run in development mode
	@export APP__ENV=dev && cargo run --bin blockchain-api

run: dev ## Alias for dev

build: ## Build debug version
	cargo build

release: ## Build release version
	cargo build --release

lint: ## Run clippy
	cargo clippy --all-targets --all-features -- -D warnings

format: ## Format code
	cargo fmt --all

test: ## Run tests
	cargo test --all

check: format lint ## Format and lint

clean: ## Clean build artifacts
	cargo clean

watch: ## Run with auto-reload (requires cargo-watch)
	cargo watch -x 'run --bin blockchain-api'


