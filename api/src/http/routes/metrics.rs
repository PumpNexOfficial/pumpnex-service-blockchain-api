use actix_web::{HttpResponse, Responder};

pub async fn get_metrics() -> impl Responder {
    // Simplified metrics endpoint
    // In a full implementation, this would return Prometheus metrics
    let metrics = "# HELP blockchain_api_info Information about the blockchain API
# TYPE blockchain_api_info gauge
blockchain_api_info{version=\"0.2.0\"} 1

# HELP http_requests_total Total number of HTTP requests
# TYPE http_requests_total counter
http_requests_total{method=\"GET\",path=\"/healthz\",status=\"200\"} 1
";

    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(metrics)
}