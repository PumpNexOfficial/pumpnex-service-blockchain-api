# Grafana Dashboards for Blockchain API

This document describes the recommended Grafana dashboards for monitoring the Blockchain API service.

## Dashboard Structure

### 1. API Overview Dashboard

**Purpose**: High-level overview of API performance and health

**Panels**:
- **Request Rate (RPS)**: `rate(http_requests_total[1m])`
- **Response Time P95**: `histogram_quantile(0.95, sum by (le) (rate(http_request_duration_seconds_bucket[5m])))`
- **Response Time P99**: `histogram_quantile(0.99, sum by (le) (rate(http_request_duration_seconds_bucket[5m])))`
- **Error Rate**: `rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m])`
- **Status Code Distribution**: `sum by (status) (rate(http_requests_total[5m]))`
- **Active Circuit Breakers**: `circuit_state_changes{state="OPEN"}`
- **Request Duration by Endpoint**: `histogram_quantile(0.95, sum by (le, path) (rate(http_request_duration_seconds_bucket[5m])))`

**SLO Targets**:
- P95 latency < 100ms for health endpoints
- P95 latency < 350ms for API endpoints
- Error rate < 0.5%

### 2. WebSocket Dashboard

**Purpose**: Monitor WebSocket connections and events

**Panels**:
- **Active Connections**: `ws_connections_active`
- **Events per Second**: `rate(ws_events_total[1m])`
- **Event Types**: `sum by (event) (rate(ws_events_total[5m]))`
- **Connection Duration**: `histogram_quantile(0.95, sum by (le) (rate(ws_connection_duration_seconds_bucket[5m])))`
- **Disconnect Rate**: `rate(ws_events_total{event="disconnect"}[5m])`
- **Backpressure Events**: `rate(ws_events_total{event="backpressure"}[5m])`

**SLO Targets**:
- WebSocket availability ≥ 99%
- Connection time < 1s
- Event delivery latency < 100ms

### 3. Ingestion Dashboard

**Purpose**: Monitor Kafka ingestion and processing

**Panels**:
- **Messages per Second**: `rate(ingest_messages_received_total[1m])`
- **Batch Processing Duration**: `histogram_quantile(0.95, sum by (le) (rate(ingest_batch_duration_seconds_bucket[5m])))`
- **Processing Errors**: `rate(ingest_errors_total[5m])`
- **DLQ Rate**: `rate(ingest_dlq_messages_total[5m])`
- **Queue Depth**: `kafka_consumer_lag_sum`
- **Processing Success Rate**: `rate(ingest_messages_processed_total[5m]) / rate(ingest_messages_received_total[5m])`

**SLO Targets**:
- Batch processing latency < 5s
- DLQ rate < 1%
- Processing success rate > 99%

### 4. Database & Cache Dashboard

**Purpose**: Monitor database and Redis performance

**Panels**:
- **Database QPS**: `rate(db_queries_total[1m])`
- **Database Latency**: `histogram_quantile(0.95, sum by (le) (rate(db_query_duration_seconds_bucket[5m])))`
- **Connection Pool Usage**: `pg_stat_activity_count / pg_settings_max_connections`
- **Redis Operations**: `rate(redis_operations_total[1m])`
- **Redis Latency**: `histogram_quantile(0.95, sum by (le) (rate(redis_operation_duration_seconds_bucket[5m])))`
- **Cache Hit Rate**: `rate(cache_hits_total[5m]) / rate(cache_operations_total[5m])`

**SLO Targets**:
- Database P95 latency < 50ms
- Redis P95 latency < 10ms
- Cache hit rate > 80%

### 5. Infrastructure Dashboard

**Purpose**: Monitor system resources and infrastructure

**Panels**:
- **CPU Usage**: `rate(process_cpu_seconds_total[5m])`
- **Memory Usage**: `process_resident_memory_bytes / machine_memory_bytes`
- **Goroutines**: `go_goroutines`
- **GC Duration**: `histogram_quantile(0.95, sum by (le) (rate(go_gc_duration_seconds_bucket[5m])))`
- **Thread Count**: `go_threads`
- **File Descriptors**: `process_open_fds`

**SLO Targets**:
- CPU usage < 80%
- Memory usage < 80%
- GC pause < 10ms

## Dashboard Configuration

### Variables
- `$service`: Service name (default: blockchain-api)
- `$instance`: Instance selector
- `$time_range`: Time range selector

### Refresh Intervals
- **API Overview**: 30s
- **WebSocket**: 10s
- **Ingestion**: 30s
- **Database & Cache**: 30s
- **Infrastructure**: 1m

### Alerting
Each dashboard should include:
- SLO status indicators
- Alert state panels
- Runbook links
- Escalation procedures

## SLO Definitions

### Availability SLO
- **Target**: 99.9% availability
- **Measurement**: `up{job="blockchain-api"}`
- **Window**: 30 days

### Latency SLO
- **Health endpoints**: P95 < 100ms, P99 < 250ms
- **API endpoints**: P95 < 350ms, P99 < 500ms
- **WebSocket**: P95 < 100ms, P99 < 200ms

### Error Rate SLO
- **Target**: < 0.5% error rate
- **Measurement**: `rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m])`

## Runbooks

### High Error Rate
1. Check application logs for errors
2. Verify database connectivity
3. Check Redis connectivity
4. Review recent deployments
5. Scale horizontally if needed

### High Latency
1. Check database performance
2. Review Redis performance
3. Check network connectivity
4. Review resource usage
5. Consider connection pool tuning

### Circuit Breaker Open
1. Check target service health
2. Review error logs
3. Verify network connectivity
4. Wait for automatic recovery
5. Manual intervention if needed

### WebSocket Issues
1. Check WebSocket connection logs
2. Verify event processing
3. Review rate limiting
4. Check client connectivity
5. Scale WebSocket workers if needed

## Dashboard JSON Export

To export dashboard configurations:

```bash
# Export API Overview dashboard
curl -H "Authorization: Bearer $GRAFANA_TOKEN" \
  "http://grafana:3000/api/dashboards/uid/api-overview" \
  | jq '.dashboard' > api-overview.json

# Export WebSocket dashboard
curl -H "Authorization: Bearer $GRAFANA_TOKEN" \
  "http://grafana:3000/api/dashboards/uid/websocket" \
  | jq '.dashboard' > websocket.json
```

## Monitoring Best Practices

1. **Set up alerts** for all SLO violations
2. **Use consistent time ranges** across dashboards
3. **Include runbook links** in alert annotations
4. **Regular review** of dashboard effectiveness
5. **Update thresholds** based on actual performance
6. **Document escalation procedures**
7. **Test alerting** regularly

