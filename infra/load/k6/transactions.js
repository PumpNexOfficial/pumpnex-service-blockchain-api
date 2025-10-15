import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('error_rate');
const transactionLatency = new Trend('transaction_latency');

// Configuration from environment
const BASE_URL = __ENV.BASE_URL || 'http://127.0.0.1:8080';
const METRICS_URL = __ENV.METRICS_URL || BASE_URL + '/metrics';
const DURATION = __ENV.DURATION || '30s';
const VUS = __ENV.VUS || '10';

// Helper function to build query string
function qs(params) {
  const pairs = [];
  for (const key in params) {
    if (params.hasOwnProperty(key)) {
      pairs.push(encodeURIComponent(key) + '=' + encodeURIComponent(params[key]));
    }
  }
  return pairs.length > 0 ? '?' + pairs.join('&') : '';
}

export let options = {
  stages: [
    { duration: '10s', target: parseInt(VUS) * 0.5 }, // Ramp up
    { duration: DURATION, target: parseInt(VUS) },     // Steady state
    { duration: '10s', target: parseInt(VUS) * 2 },  // Spike
    { duration: '10s', target: parseInt(VUS) },       // Back to steady
    { duration: '10s', target: 0 },                   // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<100', 'p(99)<250'], // SLO targets
    error_rate: ['rate<0.005'],                    // <0.5% error rate
    transaction_latency: ['p(95)<100', 'p(99)<250'],
  },
};

export default function () {
  // Test health endpoint
  let healthResponse = http.get(`${BASE_URL}/healthz`);
  let healthCheck = check(healthResponse, {
    'health status is 200': (r) => r.status === 200,
    'health response time < 50ms': (r) => r.timings.duration < 50,
  });
  errorRate.add(!healthCheck);
  transactionLatency.add(healthResponse.timings.duration);

  // Test version endpoint
  let versionResponse = http.get(`${BASE_URL}/version`);
  let versionCheck = check(versionResponse, {
    'version status is 200': (r) => r.status === 200,
    'version response time < 50ms': (r) => r.timings.duration < 50,
  });
  errorRate.add(!versionCheck);
  transactionLatency.add(versionResponse.timings.duration);

  // Test transactions API without filters
  let transactionsResponse = http.get(`${BASE_URL}/api/transactions`);
  let transactionsCheck = check(transactionsResponse, {
    'transactions status is 200': (r) => r.status === 200,
    'transactions response time < 200ms': (r) => r.timings.duration < 200,
  });
  errorRate.add(!transactionsCheck);
  transactionLatency.add(transactionsResponse.timings.duration);

  // Test with filters using query string
  let filterParams = {
    limit: '10',
    offset: '0',
    sort: 'created_at',
    order: 'desc',
  };
  let queryString = qs(filterParams);
  
  let filteredResponse = http.get(`${BASE_URL}/api/transactions${queryString}`);
  let filteredCheck = check(filteredResponse, {
    'filtered transactions status is 200': (r) => r.status === 200,
    'filtered transactions response time < 300ms': (r) => r.timings.duration < 300,
  });
  errorRate.add(!filteredCheck);
  transactionLatency.add(filteredResponse.timings.duration);

  // Test metrics endpoint
  let metricsResponse = http.get(METRICS_URL);
  let metricsCheck = check(metricsResponse, {
    'metrics status is 200': (r) => r.status === 200,
    'metrics response time < 100ms': (r) => r.timings.duration < 100,
  });
  errorRate.add(!metricsCheck);
  transactionLatency.add(metricsResponse.timings.duration);

  sleep(0.1); // 100ms between requests
}

export function handleSummary(data) {
  const duration = data.metrics.iteration_duration && data.metrics.iteration_duration.values ? data.metrics.iteration_duration.values.avg : 0;
  const rps = data.metrics.http_reqs && data.metrics.http_reqs.values ? data.metrics.http_reqs.values.rate : 0;
  const p95 = data.metrics.http_req_duration && data.metrics.http_req_duration.values ? data.metrics.http_req_duration.values['p(95)'] : 0;
  const p99 = data.metrics.http_req_duration && data.metrics.http_req_duration.values ? data.metrics.http_req_duration.values['p(99)'] : 0;
  const errorRate = data.metrics.error_rate && data.metrics.error_rate.values ? data.metrics.error_rate.values.rate * 100 : 0;
  const checks = data.metrics.checks && data.metrics.checks.values ? data.metrics.checks.values.rate * 100 : 0;
  
  return {
    'summary.json': JSON.stringify(data, null, 2),
    stdout: `
=== Load Test Summary ===
Duration: ${duration.toFixed(2)}ms avg
RPS: ${rps.toFixed(2)} req/s
P95: ${p95.toFixed(2)}ms
P99: ${p99.toFixed(2)}ms
Error Rate: ${errorRate.toFixed(2)}%
Checks: ${checks.toFixed(2)}%
`,
  };
}
