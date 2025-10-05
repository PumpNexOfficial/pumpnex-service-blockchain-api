use serde::{Deserialize, Serialize};
use validator::Validate;
use std::sync::Arc;
use prometheus::{IntCounterVec, Registry};
use thiserror::Error;
use actix_web::http::StatusCode;
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct SolanaTransaction {
    pub id: uuid::Uuid,
    pub signature: String,
    #[validate(length(min = 1))]
    pub from_pubkey: String,
    pub to_pubkey: Option<String>,
    pub instructions: serde_json::Value,
    pub lamports: Option<i64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub slot: Option<i64>,
}
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct NonceRequest {
    #[validate(length(min = 1))]
    pub wallet_address: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct NonceResponse {
    pub nonce: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct SolanaTransactionFilter {
    pub from_pubkey: Option<String>,
    pub to_pubkey: Option<String>,
    pub min_lamports: Option<i64>,
    pub start_slot: Option<i64>,
    pub end_slot: Option<i64>,
    pub program_id: Option<String>,
    pub instruction_type: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct PaginationQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
    pub sort_by: Option<String>,
    pub order: Option<String>,
}
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Too Many Requests: {0}")]
    TooManyRequests(String),
    #[error("Internal Server Error: {0}")]
    InternalServerError(String),
    #[error("Cache Error: {0}")]
    CacheError(String),
    #[error("Serialization Error: {0}")]
    SerializationError(String),
}
impl actix_web::error::ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    fn error_response(&self) -> actix_web::HttpResponse {
        actix_web::HttpResponse::build(self.status_code())
            .json(ErrorResponse { error: self.to_string() })
    }
}
#[derive(Clone)]
pub struct Metrics {
    pub requests: IntCounterVec,
    pub cache_hits: IntCounterVec,
    pub rate_limit_exceeded: IntCounterVec,
}
impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();
        let requests = IntCounterVec::new(
            prometheus::Opts::new("requests_total", "Total HTTP requests").into(),
            &["endpoint"]
        ).expect("metrics");
        let cache_hits = IntCounterVec::new(
            prometheus::Opts::new("cache_hits_total", "Total cache hits").into(),
            &["cache_type"]
        ).expect("metrics");
        let rate_limit_exceeded = IntCounterVec::new(
            prometheus::Opts::new("rate_limit_exceeded_total", "Rate limit exceeded").into(),
            &["endpoint"]
        ).expect("metrics");
        for m in [&requests, &cache_hits, &rate_limit_exceeded] {
            registry.register(Box::new(m.clone())).expect("register");
        }
        Self { requests, cache_hits, rate_limit_exceeded }
    }
}
#[derive(Clone)]
pub struct AppState {
    pub redis: deadpool_redis::Pool,
    pub db: sqlx::PgPool,
    pub config: Arc<AppConfig>,
    pub metrics: Metrics,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub cache: CacheConfig,
    pub rate_limit: RateLimitConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub kafka: KafkaConfig,
    pub server: ServerConfig,
    pub security: SecurityConfig,
    pub websocket: WebSocketConfig,
    pub pagination: PaginationConfig,
    pub circuit_breaker: CircuitBreakerConfig,
    pub retry: RetryConfig,
    pub solana: SolanaConfig,
}
impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            cache: CacheConfig {
                nonce_ttl_secs: 120,
                redis_cache_capacity: 100,
                api_data_ttl_secs: 300,
                solana_data_ttl_secs: 600,
            },
            rate_limit: RateLimitConfig {
                ip_max_requests: 100,
                ip_window_secs: 60,
                user_max_requests: 10,
                user_window_secs: 60,
                admins_exempt: true,
            },
            database: DatabaseConfig {
                url: "postgres://postgres:postgres@localhost:5432/test".into(),
                max_connections: 5,
                acquire_timeout_secs: 5,
                idle_timeout_secs: 30,
                run_migrations: false,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".into(),
            },
            kafka: KafkaConfig {
                bootstrap_servers: "localhost:9092".into(),
            },
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 8080,
                tls_cert_path: "".into(),
                tls_key_path: "".into(),
                max_request_size_mb: 5,
                log_level: "debug".into(),
            },
            security: SecurityConfig {
                waf_mode: "off".into(),
                waf_block_duration_secs: 60,
                admin_roles: vec!["admin".into()],
            },
            websocket: WebSocketConfig {
                ping_interval_secs: 10,
                client_timeout_secs: 30,
                max_subscriptions_per_user: 10,
                subscription_ttl_secs: 60,
                allowed_event_types: vec!["tx".into()],
            },
            pagination: PaginationConfig {
                default_limit: 10,
                max_limit: 100,
            },
            circuit_breaker: CircuitBreakerConfig {
                max_failures: 5,
                reset_timeout_secs: 60,
            },
            retry: RetryConfig {
                max_retries: 3,
                initial_delay_ms: 100,
            },
            solana: SolanaConfig {
                rpc_url: "http://localhost:8899".into(),
                allowed_program_ids: vec!["11111111111111111111111111111111".into()],
            },
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub nonce_ttl_secs: u64,
    pub redis_cache_capacity: u64,
    pub api_data_ttl_secs: u64,
    pub solana_data_ttl_secs: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub ip_max_requests: u64,
    pub ip_window_secs: u64,
    pub user_max_requests: u64,
    pub user_window_secs: u64,
    pub admins_exempt: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub run_migrations: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    pub bootstrap_servers: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tls_cert_path: String,
    pub tls_key_path: String,
    pub max_request_size_mb: u64,
    pub log_level: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub waf_mode: String,
    pub waf_block_duration_secs: u64,
    pub admin_roles: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub ping_interval_secs: u64,
    pub client_timeout_secs: u64,
    pub max_subscriptions_per_user: usize,
    pub subscription_ttl_secs: u64,
    pub allowed_event_types: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    pub default_limit: i64,
    pub max_limit: i64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub max_failures: u32,
    pub reset_timeout_secs: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SolanaConfig {
    pub rpc_url: String,
    pub allowed_program_ids: Vec<String>,
}
