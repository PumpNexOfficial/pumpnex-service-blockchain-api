# Blockchain API v2

A high-performance blockchain API service built with Rust and Actix Web.

## Features

- **HTTP API**: RESTful endpoints for blockchain data
- **WebSocket**: Real-time transaction streaming
- **Authentication**: Wallet-based authentication with Ed25519 signatures
- **Rate Limiting**: Configurable rate limiting with bypass paths
- **WAF**: Web Application Firewall with pattern matching
- **Caching**: Redis-based caching with adaptive strategies
- **Observability**: OpenTelemetry tracing, Prometheus metrics, structured logging
- **Security**: Security headers, CORS, TLS support
- **Database**: PostgreSQL with automatic migrations
- **Message Queue**: Kafka integration for transaction ingestion
- **Load Testing**: k6-based load testing with SLO validation
- **Performance**: Circuit breakers, timeouts, retries with jitter

## Quick Start

### Prerequisites

- Rust 1.90.0+
- PostgreSQL 15+
- Redis 7+
- Kafka 7.6.1+

### Development

1. **Clone and setup**:
   ```bash
   git clone <repository>
   cd blockchain-api
   cp .env.sample .env
   ```

2. **Start dependencies**:
   ```bash
   docker compose -f infra/docker/docker-compose.yml up -d
   ```

3. **Run migrations**:
   ```bash
   sqlx database create
   sqlx migrate run
   ```

4. **Start the service**:
   ```bash
   cargo run --bin blockchain-api
   ```

5. **Test endpoints**:
   ```bash
   curl http://localhost:8080/healthz
   curl http://localhost:8080/readyz
   curl http://localhost:8080/version
   ```

### Configuration

The service uses TOML configuration files with environment variable overrides:

- `configs/dev/default.toml` - Development settings
- `configs/stage/default.toml` - Staging settings  
- `configs/prod/default.toml` - Production settings

Environment variables use the `APP__` prefix (double underscore for nesting):
```bash
APP__SERVER__PORT=8080
APP__INTEGRATIONS__DATABASE_URL=postgres://...
```

## API Endpoints

### Health & Status
- `GET /healthz` - Health check (always returns 200)
- `GET /readyz` - Readiness check (checks dependencies)
- `GET /version` - Service version information

### Blockchain Data
- `GET /api/transactions` - List transactions with filtering
- `GET /api/transactions/{id}` - Get specific transaction
- `POST /api/transactions` - Create transaction (authenticated)

### WebSocket
- `GET /ws` - WebSocket connection for real-time updates

### Admin
- `GET /admin/waf/stats` - WAF statistics
- `POST /admin/waf/ban` - Ban IP address
- `DELETE /admin/waf/ban/{ip}` - Unban IP address

### Metrics
- `GET /metrics` - Prometheus metrics

## Development

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Format code
cargo fmt --all

# Lint code
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --all
```

### Docker

```bash
# Build image
docker build -f infra/docker/Dockerfile -t blockchain-api .

# Run container
docker run -p 8080:8080 -p 9464:9464 blockchain-api
```

### Docker Compose

```bash
# Start all services
docker compose -f infra/docker/docker-compose.yml up -d

# View logs
docker compose -f infra/docker/docker-compose.yml logs -f blockchain-api

# Stop services
docker compose -f infra/docker/docker-compose.yml down
```

## Deployment

### Kubernetes

1. **Apply manifests**:
   ```bash
   kubectl apply -f infra/k8s/
   ```

2. **Check deployment**:
   ```bash
   kubectl get pods -l app=blockchain-api
   kubectl get service blockchain-api
   ```

3. **View logs**:
   ```bash
   kubectl logs -l app=blockchain-api -f
   ```

### Rollout/Rollback

**Rollout**:
```bash
# Update image
kubectl set image deployment/blockchain-api blockchain-api=ghcr.io/OWNER/blockchain-api:v0.2.1

# Wait for rollout
kubectl rollout status deployment/blockchain-api
```

**Rollback**:
```bash
# Rollback to previous version
kubectl rollout undo deployment/blockchain-api

# Check rollout history
kubectl rollout history deployment/blockchain-api
```

### Docker Compose (Staging)

```bash
# Update image
export TAG=v0.2.1
docker compose -f infra/docker/docker-compose.yml pull
docker compose -f infra/docker/docker-compose.yml up -d

# Rollback
export TAG=v0.2.0
docker compose -f infra/docker/docker-compose.yml pull
docker compose -f infra/docker/docker-compose.yml up -d
```

## Monitoring

### Health Checks

- **Liveness**: `GET /healthz` - Process health
- **Readiness**: `GET /readyz` - Dependencies health
- **Metrics**: `GET /metrics` - Prometheus metrics

### Logs

Structured JSON logs with correlation IDs:
```json
{
  "timestamp": "2024-01-01T00:00:00Z",
  "level": "info",
  "request_id": "req-123",
  "trace_id": "trace-456",
  "span_id": "span-789",
  "message": "Request completed",
  "method": "GET",
  "path": "/api/transactions",
  "status": 200,
  "duration_ms": 45
}
```

### Metrics

Key metrics exposed at `/metrics`:
- `http_requests_total` - HTTP request counter
- `http_request_duration_seconds` - Request duration histogram
- `ws_events_total` - WebSocket event counter
- `cache_hits_total` - Cache hit counter
- `cache_miss_total` - Cache miss counter
- `ingest_batch_duration_seconds` - Ingestion batch duration
- `db_query_duration_seconds` - Database query duration

## Security

### Authentication

Wallet-based authentication using Ed25519 signatures:
1. Client requests nonce via `GET /auth/nonce`
2. Client signs `METHOD:PATH:NONCE` with wallet private key
3. Client includes signature in `Authorization` header
4. Server verifies signature and nonce validity

### Rate Limiting

Configurable rate limiting with bypass paths:
- Default: 100 requests per 60 seconds
- Bypass paths: `/healthz`, `/readyz`, `/version`, `/metrics`
- IP-based and user-based limits

### WAF (Web Application Firewall)

Pattern-based request analysis:
- SQL injection detection
- XSS pattern matching
- Path traversal detection
- Configurable scoring and blocking

### Security Headers

- `Strict-Transport-Security` (HSTS)
- `X-Frame-Options: DENY`
- `X-Content-Type-Options: nosniff`
- `Referrer-Policy: no-referrer`
- `Permissions-Policy`
- `Content-Security-Policy`

## Architecture

### Components

- **HTTP Server**: Actix Web with middleware pipeline
- **WebSocket**: Real-time transaction streaming
- **Database**: PostgreSQL with connection pooling
- **Cache**: Redis with adaptive strategies
- **Message Queue**: Kafka for transaction ingestion
- **Observability**: OpenTelemetry + Prometheus + Sentry

### Middleware Pipeline

```
RequestID → WAF → SecurityHeaders → RateLimit → WalletAuth → Logger → CORS → BodyLimit → Router
```

### Data Flow

1. **HTTP Request** → Middleware pipeline → Route handler
2. **WebSocket** → Connection manager → Event broadcasting
3. **Kafka** → Consumer → Batch processing → Database
4. **Database** → Connection pool → Query execution
5. **Cache** → Redis → Adaptive caching strategy

## Configuration Reference

### Server
```toml
[server]
host = "0.0.0.0"
port = 8080
tls_enabled = false
request_body_limit_bytes = 1048576
```

### Integrations
```toml
[integrations]
enable_postgres = true
enable_redis = true
enable_kafka = true
database_url = "postgres://..."
redis_url = "redis://..."
kafka_brokers = "127.0.0.1:9092"
```

### Security
```toml
[security]
cors_allowed_origins = ["*"]
hsts_enabled = false
frame_options = "DENY"
csp_enabled = false
```

### Observability
```toml
[telemetry]
log_format = "json"
log_level = "info"
request_id_header = "x-request-id"

[otel]
enabled = true
service_name = "blockchain-api"
traces_exporter = "otlp"
metrics_exporter = "prometheus"

[sentry]
enabled = false
environment = "dev"
```

## Troubleshooting

### Common Issues

1. **Database connection failed**:
   - Check `DATABASE_URL` environment variable
   - Verify PostgreSQL is running
   - Check network connectivity

2. **Redis connection failed**:
   - Check `REDIS_URL` environment variable
   - Verify Redis is running
   - Check network connectivity

3. **Kafka connection failed**:
   - Check `KAFKA_BROKERS` environment variable
   - Verify Kafka is running
   - Check network connectivity

4. **TLS certificate errors**:
   - Verify certificate files exist
   - Check file permissions
   - Validate certificate format

### Debug Mode

Enable debug logging:
```bash
APP__TELEMETRY__LOG_LEVEL=debug cargo run --bin blockchain-api
```

### Health Check Failures

Check service status:
```bash
curl -v http://localhost:8080/readyz
```

Check logs for specific errors:
```bash
docker logs blockchain-api
kubectl logs -l app=blockchain-api
```

## Performance & SLO

### Service Level Objectives (SLO)

- **Availability**: 99.9% uptime
- **Latency**: 
  - Health endpoints: P95 < 100ms, P99 < 250ms
  - API endpoints: P95 < 350ms, P99 < 500ms
  - WebSocket: P95 < 100ms, P99 < 200ms
- **Error Rate**: < 0.5% for public endpoints
- **WebSocket**: ≥ 99% connection success rate

### Load Testing

Run load tests to validate SLO targets:

```bash
# Install k6
curl https://github.com/grafana/k6/releases/download/v0.47.0/k6-v0.47.0-linux-amd64.tar.gz -L | tar xvz --strip-components 1

# Run load tests
k6 run infra/load/k6/transactions.js
k6 run infra/load/k6/ws_tx.js
k6 run infra/load/k6/bad_actor.js
```

### Performance Tuning

Key configuration parameters:

```toml
[server]
workers = 0  # 0 = auto (num_cpus)

[runtime]
worker_threads = 0  # 0 = auto
max_blocking_threads = 512

[integrations]
pg_max_connections = 10  # Adjust based on load
redis_max_connections = 10
```

**Tuning Recommendations**:
- **CPU-bound**: `workers = num_cpus()`
- **I/O-bound**: `workers = num_cpus() * 2`
- **Database pool**: `(workers * 2) + 5`
- **Redis pool**: 5-20 connections
- **Kafka**: `max_poll_records = 100-1000`

### Monitoring

- **Metrics**: http://localhost:8080/metrics
- **Health**: http://localhost:8080/healthz
- **Readiness**: http://localhost:8080/readyz

Key metrics to monitor:
- `http_requests_total` - Request rate
- `http_request_duration_seconds` - Response time
- `circuit_state_changes` - Circuit breaker status
- `ws_events_total` - WebSocket events
- `ingest_batch_duration_seconds` - Ingestion performance

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and linting
5. Submit a pull request

### Development Workflow

1. **Setup**: `cargo build`
2. **Test**: `cargo test --all`
3. **Format**: `cargo fmt --all`
4. **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`
5. **Commit**: Follow conventional commits

## License

[License information]
**Load test:** run `infra/load/bin/install-k6.sh` then `./k6 run infra/load/k6/transactions.js`
