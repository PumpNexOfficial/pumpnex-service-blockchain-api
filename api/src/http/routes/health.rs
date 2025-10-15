/// Health check routes

use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;
use std::collections::HashMap;

use crate::app_state::AppState;
use crate::infra::{postgres, redis, kafka};

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Serialize)]
struct ReadyResponse {
    ready: bool,
    checks: HashMap<String, CheckResult>,
}

#[derive(Serialize)]
struct CheckResult {
    enabled: bool,
    ok: bool,
    details: String,
}

pub async fn healthz() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
    })
}

pub async fn readyz(state: web::Data<AppState>) -> impl Responder {
    let mut checks = HashMap::new();
    let mut overall_ready = true;

    // Check Postgres
    if let Some(ref pool) = state.postgres {
        match postgres::check_postgres_health(pool).await {
            Ok(_) => {
                checks.insert(
                    "postgres".to_string(),
                    CheckResult {
                        enabled: true,
                        ok: true,
                        details: "healthy".to_string(),
                    },
                );
            }
            Err(e) => {
                overall_ready = false;
                checks.insert(
                    "postgres".to_string(),
                    CheckResult {
                        enabled: true,
                        ok: false,
                        details: e,
                    },
                );
            }
        }
    } else {
        checks.insert(
            "postgres".to_string(),
            CheckResult {
                enabled: false,
                ok: true,
                details: "disabled".to_string(),
            },
        );
    }

    // Check Redis
    if let Some(redis_conn) = state.redis.clone() {
        let mut conn = redis_conn;
        match redis::check_redis_health(&mut conn).await {
            Ok(_) => {
                checks.insert(
                    "redis".to_string(),
                    CheckResult {
                        enabled: true,
                        ok: true,
                        details: "healthy".to_string(),
                    },
                );
            }
            Err(e) => {
                overall_ready = false;
                checks.insert(
                    "redis".to_string(),
                    CheckResult {
                        enabled: true,
                        ok: false,
                        details: e,
                    },
                );
            }
        }
    } else {
        checks.insert(
            "redis".to_string(),
            CheckResult {
                enabled: false,
                ok: true,
                details: "disabled".to_string(),
            },
        );
    }

    // Kafka check - not in AppState due to Clone limitation
    checks.insert(
        "kafka".to_string(),
        CheckResult {
            enabled: false,
            ok: true,
            details: "disabled or not checked".to_string(),
        },
    );

    let status_code = if overall_ready { 200 } else { 503 };

    HttpResponse::build(actix_web::http::StatusCode::from_u16(status_code).unwrap())
        .json(ReadyResponse {
            ready: overall_ready,
            checks,
        })
}
