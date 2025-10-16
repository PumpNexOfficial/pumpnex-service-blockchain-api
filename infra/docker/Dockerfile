# Multi-stage build for blockchain-api
FROM rust:1.90-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY api/Cargo.toml ./api/

# Create dummy main.rs to cache dependencies
RUN mkdir -p api/src && echo "fn main() {}" > api/src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release -p blockchain-api

# Copy source code
COPY api/src/ ./api/src/
COPY configs/ ./configs/
COPY migrations/ ./migrations/

# Build the application
RUN cargo build --release -p blockchain-api

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r blockchain && useradd -r -g blockchain blockchain

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/blockchain-api /app/blockchain-api

# Copy configs
COPY --from=builder /app/configs/ ./configs/

# Create directories for TLS certificates
RUN mkdir -p /etc/blockchain-api/tls

# Change ownership to non-root user
RUN chown -R blockchain:blockchain /app /etc/blockchain-api

# Switch to non-root user
USER blockchain

# Expose ports
EXPOSE 8080 9464

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/healthz || exit 1

# Run the application
CMD ["./blockchain-api"]

