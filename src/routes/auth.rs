use actix_web::{post, web, HttpResponse, Error as ActixError};
use tracing::{info, error};
use deadpool_redis::redis::AsyncCommands;
use uuid::Uuid;
use validator::Validate;
use crate::models::{AppError, AppState, NonceRequest, NonceResponse};

#[post("/api/auth/nonce")]
pub async fn get_nonce(
    state: web::Data<AppState>,
    body: web::Json<NonceRequest>,
) -> Result<HttpResponse, ActixError> {
    if body.0.validate().is_err() {
        error!("Invalid wallet address: {}", body.wallet_address);
        return Err(AppError::BadRequest("Invalid wallet address".to_string()).into());
    }
    let mut conn = state.redis.get().await
        .map_err(|e| AppError::RedisError(format!("Failed to get Redis connection: {}", e)))?;
    let nonce_value = Uuid::new_v4().to_string();
    let redis_key = format!("nonce:{}", body.wallet_address);
    info!("Saving nonce for key: {}, value: {}", redis_key, nonce_value);
    conn.set_ex::<_, _, ()>(&redis_key, &nonce_value, 120)
        .await
        .map_err(|e| {
            error!("Failed to set nonce in Redis for key {}: {}", redis_key, e);
            AppError::RedisError(format!("Failed to set nonce: {}", e))
        })?;
    info!("Generated nonce for wallet: {}", body.wallet_address);
    Ok(HttpResponse::Ok().json(NonceResponse { nonce: nonce_value }))
}
