/// Authentication routes

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::config::AuthConfig;

#[derive(Deserialize)]
pub struct NonceRequest {
    address: String,
}

#[derive(Serialize)]
pub struct NonceResponse {
    nonce: String,
    ttl_secs: u64,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

pub async fn get_nonce(
    req: web::Json<NonceRequest>,
    state: web::Data<AppState>,
    config: web::Data<AuthConfig>,
) -> impl Responder {
    let address = &req.address;

    // Basic validation: check if address looks like base58
    if address.len() < 32 || address.len() > 44 {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "bad_request".to_string(),
            details: Some("Invalid address format".to_string()),
        });
    }

    // Validate address is valid base58
    if let Err(e) = blockchain_auth::decode_pubkey_b58(address) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "bad_request".to_string(),
            details: Some(format!("Invalid address: {}", e)),
        });
    }

    // Generate nonce
    let nonce = blockchain_auth::generate_nonce();

    // Store in Redis (if available)
    if let Some(ref redis_conn) = state.redis {
        let redis_key = format!("{}:{}", config.redis_key_prefix, address);
        let ttl = config.nonce_ttl_secs as i64;

        let mut conn = redis_conn.clone();
        match redis::cmd("SETEX")
            .arg(&redis_key)
            .arg(ttl)
            .arg(&nonce)
            .query_async::<String>(&mut conn)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    address = %address,
                    ttl_secs = ttl,
                    redis_key = %redis_key,
                    "Nonce generated and stored"
                );
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to store nonce in Redis");
                return HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "internal".to_string(),
                    details: Some("Redis unavailable".to_string()),
                });
            }
        }
    } else {
        tracing::warn!("Redis not available, nonce verification will not work");
        return HttpResponse::ServiceUnavailable().json(ErrorResponse {
            error: "service_unavailable".to_string(),
            details: Some("Authentication service requires Redis".to_string()),
        });
    }

    HttpResponse::Ok().json(NonceResponse {
        nonce,
        ttl_secs: config.nonce_ttl_secs,
    })
}

