# PumpNex Blockchain API (v0.24.0)

High-performance Rust API for Solana blockchain data, with real-time notifications, WebSocket support, and secure wallet authentication.

## Features
- Query Solana transactions with filters (pubkey, lamports, slot).
- Nonce-based authentication with Solana signatures.
- Redis caching and PostgreSQL persistence.
- Actix-web with middleware (rate limiting, logging).
- Planned: WebSocket events, Kafka integration, Prometheus metrics.

## Setup
1. Clone: `git clone https://github.com/PumpNexOfficial/pumpnex-service-blockchain-api`
2. Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
3. Build: `cargo build --release`
4. Run: `cargo run` (uses config/default.toml; set APP_DATABASE_URL, APP_REDIS_URL env vars).

## Progress
- Iteration 1: Models and config aligned (UUID IDs, Option<i64> fields, simd-json validation).
- Iteration 2: Authentication refactor (in progress).

License: MIT. Contributions welcome!
🔥 v0.25.0 — обновлено Mon Oct 13 03:19:56 PM CEST 2025
