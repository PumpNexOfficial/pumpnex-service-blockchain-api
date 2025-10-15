import ws from 'k6/ws';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const wsErrorRate = new Rate('ws_error_rate');
const wsLatency = new Trend('ws_latency');
const wsEventsReceived = new Counter('ws_events_received');
const wsConnectionDuration = new Trend('ws_connection_duration');

// Configuration from environment
const BASE_URL = __ENV.BASE_URL || 'http://127.0.0.1:8080';
const DURATION = __ENV.DURATION || '30s';
const VUS = __ENV.VUS || '5';
const WS_EVENTS_RATE = __ENV.WS_EVENTS_RATE || '1'; // events per second per connection

export let options = {
  stages: [
    { duration: '5s', target: Math.floor(parseInt(VUS) * 0.5) },  // Ramp up
    { duration: DURATION, target: parseInt(VUS) },    // Steady state
    { duration: '5s', target: parseInt(VUS) * 2 },    // Spike
    { duration: '5s', target: parseInt(VUS) },        // Back to steady
    { duration: '5s', target: 0 },                    // Ramp down
  ],
  thresholds: {
    ws_connection_duration: ['p(95)<1000'],           // Connection time < 1s
    ws_error_rate: ['rate<0.01'],                    // <1% error rate
    ws_events_received: ['count>0'],                 // Must receive events
  },
};

export default function () {
  const url = `${BASE_URL.replace('http', 'ws')}/ws/tx`;
  const startTime = Date.now();
  
  const res = ws.connect(url, {}, function (socket) {
    let eventCount = 0;
    const maxEvents = parseInt(WS_EVENTS_RATE) * 10; // 10 seconds worth of events
    
    socket.on('open', function () {
      const connectionTime = Date.now() - startTime;
      wsConnectionDuration.add(connectionTime);
      
      // Subscribe to transaction events
      const subscribeMessage = JSON.stringify({
        type: 'subscribe',
        filters: {
          account: 'all',
          program: 'all',
        },
      });
      
      socket.send(subscribeMessage);
    });

    socket.on('message', function (data) {
      try {
        const message = JSON.parse(data);
        eventCount++;
        wsEventsReceived.add(1);
        
        // Check message structure
        const messageCheck = check(message, {
          'message has type': (m) => m.type !== undefined,
          'message has data': (m) => m.data !== undefined,
        });
        
        if (!messageCheck) {
          wsErrorRate.add(1);
        }
        
        // Rate limit: don't process too many events
        if (eventCount >= maxEvents) {
          socket.close();
        }
      } catch (e) {
        wsErrorRate.add(1);
      }
    });

    socket.on('close', function () {
      const totalDuration = Date.now() - startTime;
      wsLatency.add(totalDuration);
    });

    socket.on('error', function (e) {
      wsErrorRate.add(1);
    });

    // Keep connection alive for a while
    sleep(10);
    socket.close();
  });

  const connectionCheck = check(res, {
    'websocket connection successful': (r) => r === undefined,
  });
  
  if (!connectionCheck) {
    wsErrorRate.add(1);
  }
}

export function handleSummary(data) {
  const connections = data.metrics.vus && data.metrics.vus.values ? data.metrics.vus.values.max : 0;
  const avgConnTime = data.metrics.ws_connection_duration && data.metrics.ws_connection_duration.values ? data.metrics.ws_connection_duration.values.avg : 0;
  const p95ConnTime = data.metrics.ws_connection_duration && data.metrics.ws_connection_duration.values ? data.metrics.ws_connection_duration.values['p(95)'] : 0;
  const eventsReceived = data.metrics.ws_events_received && data.metrics.ws_events_received.values ? data.metrics.ws_events_received.values.count : 0;
  const errorRate = data.metrics.ws_error_rate && data.metrics.ws_error_rate.values ? data.metrics.ws_error_rate.values.rate * 100 : 0;
  
  return {
    'ws_summary.json': JSON.stringify(data, null, 2),
    stdout: `
=== WebSocket Load Test Summary ===
Connections: ${connections}
Avg Connection Time: ${avgConnTime.toFixed(2)}ms
P95 Connection Time: ${p95ConnTime.toFixed(2)}ms
Events Received: ${eventsReceived}
Error Rate: ${errorRate.toFixed(2)}%
`,
  };
}
