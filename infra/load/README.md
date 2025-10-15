# Load Testing for Blockchain API

This directory contains load testing scripts and configurations for the Blockchain API service.

## Prerequisites

- [k6](https://k6.io/docs/getting-started/installation/) installed
- Blockchain API service running
- Optional: Kafka, Redis, PostgreSQL for full integration testing

## Test Scenarios

### 1. Transaction API Load Test (`transactions.js`)

**Purpose**: Test REST API endpoints under load

**Endpoints tested**:
- `GET /healthz` - Health check
- `GET /version` - Version info
- `GET /api/transactions` - Transaction listing
- `GET /api/transactions?filters` - Filtered transactions
- `GET /metrics` - Metrics endpoint

**SLO Targets**:
- P95 latency < 100ms for health endpoints
- P95 latency < 350ms for API endpoints
- Error rate < 0.5%

**Usage**:
```bash
# Basic test
k6 run infra/load/k6/transactions.js

# Custom parameters
BASE_URL=http://127.0.0.1:8080 VUS=30 DURATION=1m k6 run infra/load/k6/transactions.js

# With separate metrics endpoint
METRICS_URL=http://127.0.0.1:9464/metrics k6 run infra/load/k6/transactions.js

# Generate detailed report
k6 run --out json=results.json infra/load/k6/transactions.js
```

### 2. WebSocket Load Test (`ws_tx.js`)

**Purpose**: Test WebSocket connections and event streaming

**Features tested**:
- WebSocket connection establishment
- Event subscription
- Message processing
- Connection stability

**SLO Targets**:
- Connection time < 1s
- Event delivery latency < 100ms
- Connection success rate > 99%

**Usage**:
```bash
# Basic WebSocket test
k6 run infra/load/k6/ws_tx.js

# High event rate test
WS_EVENTS_RATE=5 VUS=10 k6 run infra/load/k6/ws_tx.js

# Long duration test
DURATION=300s VUS=5 k6 run infra/load/k6/ws_tx.js
```

### 3. Bad Actor Load Test (`bad_actor.js`)

**Purpose**: Test security features and rate limiting

**Attack patterns tested**:
- Rapid fire requests (rate limiting)
- Malicious payloads (WAF)
- Large payloads (body size limits)
- SQL injection attempts
- XSS attempts

**Expected behavior**:
- Rate limiting triggers (429 responses)
- WAF blocks malicious requests (403 responses)
- Body size limits enforced (413 responses)
- Legitimate traffic continues to work

**Usage**:
```bash
# Basic security test
k6 run infra/load/k6/bad_actor.js

# Aggressive attack simulation
VUS=20 DURATION=60s k6 run infra/load/k6/bad_actor.js
```

## Running Load Tests

### Local Development

```bash
# Start the API service
cd /root/pumpnex-services/blockchain-api
APP__ENV=dev cargo run --release --bin blockchain-api

# In another terminal, run load tests
cd infra/load

# Quick smoke test
k6 run k6/transactions.js

# Test with separate metrics endpoint
METRICS_URL=http://localhost:9464/metrics k6 run k6/transactions.js

# Comprehensive test suite
k6 run k6/transactions.js && \
k6 run k6/ws_tx.js && \
k6 run k6/bad_actor.js
```

### CI/CD Integration

```yaml
# .github/workflows/load-test.yml
name: Load Testing
on:
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM
  workflow_dispatch:

jobs:
  load-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install k6
        run: |
          sudo apt-key adv --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
          echo "deb https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
          sudo apt-get update
          sudo apt-get install k6
      - name: Start services
        run: |
          docker-compose up -d
          sleep 30
      - name: Run load tests
        run: |
          k6 run infra/load/k6/transactions.js
          k6 run infra/load/k6/ws_tx.js
          k6 run infra/load/k6/bad_actor.js
```

### Performance Benchmarking

```bash
# Baseline performance test
k6 run --out json=baseline.json k6/transactions.js

# Stress test (find breaking point)
for vus in 10 20 50 100 200; do
  echo "Testing with $vus VUs"
  VUS=$vus k6 run --out json=stress_${vus}.json k6/transactions.js
done

# Compare results
k6 run --out json=current.json k6/transactions.js
# Compare baseline.json vs current.json
```

## Interpreting Results

### Key Metrics

1. **Request Rate (RPS)**: `http_reqs` rate
2. **Response Time**: `http_req_duration` percentiles
3. **Error Rate**: `http_req_failed` rate
4. **Throughput**: Successful requests per second

### SLO Validation

```bash
# Check if SLO targets are met
k6 run k6/transactions.js | grep -E "(p\(95\)|p\(99\)|error_rate)"

# Expected output:
# p(95): <100ms for health endpoints
# p(95): <350ms for API endpoints
# error_rate: <0.5%
```

### Performance Tuning

Based on load test results, adjust:

1. **Actix Workers**: `[server] workers` in config
2. **Tokio Threads**: `[runtime] worker_threads` in config
3. **Database Pool**: `[integrations] pg_max_connections`
4. **Redis Connections**: Redis connection pool settings
5. **Kafka Consumer**: `[kafka] max_poll_records`

## Tuning Recommendations

### Actix Configuration

```toml
[server]
workers = 0  # 0 = auto (num_cpus), or set specific number
```

**Recommendations**:
- CPU-bound workloads: `workers = num_cpus()`
- I/O-bound workloads: `workers = num_cpus() * 2`
- Memory-constrained: `workers = num_cpus() / 2`

### Tokio Runtime

```toml
[runtime]
worker_threads = 0  # 0 = auto
max_blocking_threads = 512
```

**Recommendations**:
- Default: `worker_threads = 0` (auto)
- High concurrency: `worker_threads = num_cpus() * 2`
- Blocking operations: increase `max_blocking_threads`

### Database Pool

```toml
[integrations]
pg_max_connections = 10
```

**Recommendations**:
- Light load: 5-10 connections
- Medium load: 10-20 connections
- Heavy load: 20-50 connections
- Formula: `(workers * 2) + 5`

### Redis Configuration

```toml
[integrations]
redis_max_connections = 10
redis_connection_timeout_ms = 1000
```

**Recommendations**:
- Connection pool: 5-20 connections
- Timeout: 1000-5000ms
- Keep-alive: enabled

### Kafka Consumer

```toml
[kafka]
max_poll_records = 100
enable_auto_commit = true
```

**Recommendations**:
- `max_poll_records`: 100-1000
- `enable_auto_commit`: true for reliability
- Batch size: 1-10MB

## Troubleshooting

### Common Issues

1. **High Latency**
   - Check database performance
   - Review Redis connectivity
   - Verify network latency
   - Check resource usage

2. **High Error Rate**
   - Check application logs
   - Verify database connectivity
   - Review Redis connectivity
   - Check rate limiting

3. **WebSocket Issues**
   - Check WebSocket connection logs
   - Verify event processing
   - Review rate limiting
   - Check client connectivity

4. **Memory Issues**
   - Check for memory leaks
   - Review connection pools
   - Verify garbage collection
   - Check resource limits

### Debug Commands

```bash
# Check service health
curl http://localhost:8080/healthz
curl http://localhost:8080/readyz

# Check metrics
curl http://localhost:8080/metrics | grep -E "(http_requests|http_duration)"

# Check WebSocket
wscat -c ws://localhost:8080/ws/tx

# Monitor resources
htop
iostat -x 1
```

## Performance Baselines

### Expected Performance (Dev Environment)

- **RPS**: 100-500 requests/second
- **P95 Latency**: <100ms (health), <350ms (API)
- **P99 Latency**: <250ms (health), <500ms (API)
- **Error Rate**: <0.5%
- **Memory Usage**: <512MB
- **CPU Usage**: <50%

### Scaling Targets

- **Small**: 100 RPS, 2 workers, 5 DB connections
- **Medium**: 500 RPS, 4 workers, 10 DB connections
- **Large**: 1000 RPS, 8 workers, 20 DB connections
- **Enterprise**: 5000+ RPS, 16+ workers, 50+ DB connections

## Continuous Monitoring

Set up continuous monitoring with:

1. **Prometheus**: Collect metrics during load tests
2. **Grafana**: Visualize performance trends
3. **Alerts**: Notify on SLO violations
4. **Reports**: Automated performance reports

```bash
# Example monitoring setup
k6 run --out prometheus=http://prometheus:9090/api/v1/write k6/transactions.js
```
