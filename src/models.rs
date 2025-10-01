use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use utoipa::ToSchema;
use regex::Regex;
use thiserror::Error;
use serde_json::Value;
use actix_web::HttpResponse;
use std::clone::Clone;
use bs58;
use solana_sdk::pubkey::Pubkey;
use sqlx::FromRow;
use simd_json::{from_str, owned::Value as SimdValue};
use std::sync::Arc;
use sqlx::PgPool;
use deadpool_redis::Pool;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Internal Server Error: {0}")]
    InternalServerError(String),
    #[error("Too Many Requests: {0}")]
    TooManyRequests(String),
    #[error("Not Found: {0}")]
    NotFound(String),
    #[error("SerializationError: {0}")]
    SerializationError(String),
    #[error("RedisError: {0}")]
    RedisError(String),
    #[error("StartupError: {0}")]
    StartupError(String),
    #[error("SignatureError: {0}")]
    SignatureError(String),
}

impl actix_web::error::ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::BadRequest(msg) => HttpResponse::BadRequest().json(serde_json::json!({"error": msg})),
            AppError::Unauthorized(msg) => HttpResponse::Unauthorized().json(serde_json::json!({"error": msg})),
            AppError::TooManyRequests(msg) => HttpResponse::TooManyRequests().json(serde_json::json!({"error": msg})),
            AppError::NotFound(msg) => HttpResponse::NotFound().json(serde_json::json!({"error": msg})),
            _ => HttpResponse::InternalServerError().finish(),
        }
    }
}

#[derive(Serialize, Deserialize, Validate, ToSchema, Clone, FromRow)]
pub struct SolanaTransaction {
    pub id: Uuid,
    #[validate(length(min = 1))]
    pub signature: String,
    #[validate(length(min = 1))]
    pub from_pubkey: String,
    #[validate(length(min = 1))]
    pub to_pubkey: Option<String>,
    pub instructions: Vec<Value>,
    pub lamports: Option<i64>,
    pub slot: Option<i64>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Validate, ToSchema, Clone)]
pub struct NonceRequest {
    #[validate(length(min = 32, max = 44))]
    pub wallet_address: String,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct NonceResponse {
    pub nonce: String,
}

#[derive(Serialize, Deserialize, Validate, ToSchema, Clone)]
pub struct Subscription {
    #[validate(length(min = 1))]
    pub id: String,
    #[validate(length(min = 1))]
    pub event_type: String,
    #[validate(length(max = 100), custom(function = "validate_filter_json"))]
    pub filter: Option<String>,
}

#[derive(Serialize, Deserialize, Validate, ToSchema, Clone)]
pub struct Event {
    #[validate(length(min = 1))]
    pub event_type: String,
    pub data: Value,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
    pub details: Option<String>,
}

#[derive(Serialize, Deserialize, Validate, ToSchema, Clone)]
pub struct SolanaTransactionFilter {
    #[validate(length(min = 1))]
    pub from_pubkey: Option<String>,
    #[validate(length(min = 1))]
    pub to_pubkey: Option<String>,
    pub min_lamports: Option<i64>,
    pub start_slot: Option<i64>,
    pub end_slot: Option<i64>,
    #[validate(custom(function = "validate_base58"))]
    pub program_id: Option<String>,
    #[validate(length(min = 1))]
    pub instruction_type: Option<String>,
}

#[derive(Serialize, Deserialize, Validate, ToSchema, Clone)]
pub struct PaginationQuery {
    #[validate(range(min = 0))]
    pub offset: Option<i64>,
    #[validate(range(min = 1))]
    pub limit: Option<i64>,
    #[validate(length(min = 1))]
    pub cursor: Option<String>,
    #[validate(length(min = 1))]
    pub sort_by: Option<String>,
    #[validate(length(min = 1))]
    pub order: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub redis: Pool,
    pub db: PgPool,
    pub config: Arc<AppConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub kafka: KafkaConfig,
    pub cache: CacheConfig,
    pub server: ServerConfig,
    pub security: SecurityConfig,
    pub websocket: WebSocketConfig,
    pub pagination: PaginationConfig,
    pub circuit_breaker: CircuitBreakerConfig,
    pub retry: RetryConfig,
    pub solana: SolanaConfig,
    pub rate_limit: RateLimitConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub run_migrations: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct KafkaConfig {
    pub bootstrap_servers: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CacheConfig {
    pub redis_cache_capacity: u64,
    pub api_data_ttl_secs: u64,
    pub solana_data_ttl_secs: u64,
    pub nonce_ttl_secs: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tls_cert_path: String,
    pub tls_key_path: String,
    pub max_request_size_mb: u64,
    pub log_level: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SecurityConfig {
    pub waf_mode: String,
    pub waf_block_duration_secs: u64,
    pub admin_roles: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WebSocketConfig {
    pub ping_interval_secs: u64,
    pub client_timeout_secs: u64,
    pub max_subscriptions_per_user: usize,
    pub subscription_ttl_secs: u64,
    pub allowed_event_types: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PaginationConfig {
    pub default_limit: i64,
    pub max_limit: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CircuitBreakerConfig {
    pub max_failures: u32,
    pub reset_timeout_secs: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SolanaConfig {
    pub rpc_url: String,
    pub allowed_program_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RateLimitConfig {
    pub ip_max_requests: u32,
    pub ip_window_secs: u64,
    pub user_max_requests: u32,
    pub user_window_secs: u64,
    pub admins_exempt: bool,
}

pub fn sanitize_input(s: &str) -> String {
    Regex::new(r"[^\w\s-]").unwrap().replace_all(s, "").to_string()
}

pub fn validate_base58(s: &Option<String>) -> Result<(), ValidationError> {
    if let Some(s) = s {
        let decoded = bs58::decode(s).into_vec().map_err(|_| ValidationError::new("Invalid Base58"))?;
        Pubkey::try_from(decoded.as_slice())
            .map_err(|_| ValidationError::new("Invalid Solana Pubkey"))?;
    }
    Ok(())
}

pub fn validate_filter_json(s: &Option<String>) -> Result<(), ValidationError> {
    if let Some(s) = s {
        let mut s_mut = s.clone();
        unsafe { from_str::<SimdValue>(&mut s_mut) }
            .map(|_| ())
            .map_err(|_| ValidationError::new("Invalid filter JSON"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonce_request_validate() {
        let valid = NonceRequest { wallet_address: "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R".to_string() };
        assert!(valid.validate().is_ok());
        let invalid = NonceRequest { wallet_address: "short".to_string() };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_base58_validate() {
        assert!(validate_base58(&Some("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R".to_string())).is_ok());
        assert!(validate_base58(&Some("invalid".to_string())).is_err());
        assert!(validate_base58(&None).is_ok());
    }

    #[test]
    fn test_filter_json_validate() {
        let valid_json = Some(r#"{"program_id": "SystemProgram"}"#.to_string());
        assert!(validate_filter_json(&valid_json).is_ok());
        let invalid_json = Some("invalid json".to_string());
        assert!(validate_filter_json(&invalid_json).is_err());
        assert!(validate_filter_json(&None).is_ok());
    }
}
