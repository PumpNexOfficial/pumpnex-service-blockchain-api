/// Configuration module
///
/// Loads configuration from TOML files and environment variables.
/// Priority: ENV > TOML > defaults
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub service: ServiceConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub telemetry: TelemetryConfig,
    #[serde(default)]
    pub integrations: IntegrationsConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub ws: WsConfig,
    #[serde(default)]
    pub kafka: KafkaConfig,
    #[serde(default)]
    pub ingest: IngestConfig,
    #[serde(default)]
    pub waf: WafConfig,
    #[serde(default)]
    pub admin: AdminConfig,
    #[serde(default)]
    pub otel: OtelConfig,
    #[serde(default)]
    pub sentry: SentryConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub db: DbConfig,
    #[serde(default)]
    pub deploy: DeployConfig,
    #[serde(default)]
    pub image: ImageConfig,
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub load: LoadConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceConfig {
    #[serde(default = "default_service_name")]
    pub name: String,
    #[serde(default = "default_service_version")]
    pub version: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub tls_enabled: bool,
    #[serde(default = "default_request_body_limit")]
    pub request_body_limit_bytes: usize,
    #[serde(default = "default_cors_allow_origins")]
    pub cors_allow_origins: Vec<String>,
    #[serde(default = "default_cors_allow_headers")]
    pub cors_allow_headers: Vec<String>,
    #[serde(default = "default_cors_allow_methods")]
    pub cors_allow_methods: Vec<String>,
    #[serde(default = "default_workers")]
    pub workers: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelemetryConfig {
    #[serde(default = "default_log_format")]
    pub log_format: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_request_id_header")]
    pub request_id_header: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_header_wallet_address")]
    pub header_wallet_address: String,
    #[serde(default = "default_header_wallet_signature")]
    pub header_wallet_signature: String,
    #[serde(default = "default_header_wallet_nonce")]
    pub header_wallet_nonce: String,
    #[serde(default = "default_nonce_ttl_secs")]
    pub nonce_ttl_secs: u64,
    #[serde(default = "default_redis_key_prefix")]
    pub redis_key_prefix: String,
    #[serde(default = "default_bypass_paths")]
    pub bypass_paths: Vec<String>,
    #[serde(default = "default_protect_prefixes")]
    pub protect_prefixes: Vec<String>,
    #[serde(default)]
    pub require_https: bool,
    #[serde(default = "default_true")]
    pub accept_signature_b58: bool,
    #[serde(default)]
    pub accept_signature_b64: bool,
    #[serde(default = "default_canonicalize_method")]
    pub canonicalize_method: String,
    #[serde(default = "default_canonicalize_path")]
    pub canonicalize_path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimitConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_rate_limit_strategy")]
    pub strategy: String,
    #[serde(default = "default_true")]
    pub respect_x_forwarded_for: bool,
    
    // IP limits
    #[serde(default = "default_ip_max_requests")]
    pub ip_max_requests: u32,
    #[serde(default = "default_ip_window_secs")]
    pub ip_window_secs: u64,
    
    // User limits
    #[serde(default = "default_user_max_requests")]
    pub user_max_requests: u32,
    #[serde(default = "default_user_window_secs")]
    pub user_window_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IntegrationsConfig {
    // Enable flags
    #[serde(default = "default_true")]
    pub enable_postgres: bool,
    #[serde(default = "default_true")]
    pub enable_redis: bool,
    #[serde(default = "default_true")]
    pub enable_kafka: bool,
    
    // Postgres
    #[serde(default)]
    pub database_url: String,
    #[serde(default = "default_pg_max_connections")]
    pub pg_max_connections: u32,
    #[serde(default = "default_pg_connect_timeout_ms")]
    pub pg_connect_timeout_ms: u64,
    #[serde(default = "default_pg_idle_timeout_ms")]
    pub pg_idle_timeout_ms: u64,
    
    // Redis
    #[serde(default)]
    pub redis_url: String,
    #[serde(default = "default_redis_connect_timeout_ms")]
    pub redis_connect_timeout_ms: u64,
    #[serde(default = "default_redis_command_timeout_ms")]
    pub redis_command_timeout_ms: u64,
    
    // Kafka
    #[serde(default)]
    pub kafka_brokers: String,
    #[serde(default = "default_kafka_client_id")]
    pub kafka_client_id: String,
    #[serde(default = "default_kafka_metadata_timeout_ms")]
    pub kafka_metadata_timeout_ms: u64,
}

// Defaults
fn default_service_name() -> String {
    "blockchain-api".to_string()
}

fn default_service_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_request_body_limit() -> usize {
    1_048_576 // 1 MiB
}

fn default_cors_allow_origins() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_cors_allow_headers() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_cors_allow_methods() -> Vec<String> {
    vec![
        "GET".to_string(),
        "POST".to_string(),
        "PUT".to_string(),
        "PATCH".to_string(),
        "DELETE".to_string(),
        "OPTIONS".to_string(),
    ]
}

fn default_log_format() -> String {
    "json".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_request_id_header() -> String {
    "x-request-id".to_string()
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_pg_max_connections() -> u32 {
    10
}

fn default_pg_connect_timeout_ms() -> u64 {
    3000
}

fn default_pg_idle_timeout_ms() -> u64 {
    300000
}

fn default_redis_connect_timeout_ms() -> u64 {
    1000
}

fn default_redis_command_timeout_ms() -> u64 {
    1000
}

fn default_kafka_client_id() -> String {
    "blockchain-api".to_string()
}

fn default_kafka_metadata_timeout_ms() -> u64 {
    1500
}

fn default_rate_limit_strategy() -> String {
    "fixed".to_string()
}

fn default_ip_max_requests() -> u32 {
    100
}

fn default_ip_window_secs() -> u64 {
    60
}

fn default_user_max_requests() -> u32 {
    200
}

fn default_user_window_secs() -> u64 {
    60
}

fn default_header_wallet_address() -> String {
    "X-Wallet-Address".to_string()
}

fn default_header_wallet_signature() -> String {
    "X-Wallet-Signature".to_string()
}

fn default_header_wallet_nonce() -> String {
    "X-Nonce".to_string()
}

fn default_nonce_ttl_secs() -> u64 {
    120
}

fn default_redis_key_prefix() -> String {
    "auth:nonce".to_string()
}

fn default_bypass_paths() -> Vec<String> {
    vec![
        "/healthz".to_string(),
        "/readyz".to_string(),
        "/version".to_string(),
        "/api/auth/nonce".to_string(),
    ]
}

fn default_protect_prefixes() -> Vec<String> {
    vec!["/api".to_string()]
}

fn default_canonicalize_method() -> String {
    "upper".to_string()
}

fn default_canonicalize_path() -> String {
    "as-is".to_string()
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: default_service_name(),
            version: default_service_version(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            tls_enabled: false,
            request_body_limit_bytes: default_request_body_limit(),
            cors_allow_origins: default_cors_allow_origins(),
            cors_allow_headers: default_cors_allow_headers(),
            cors_allow_methods: default_cors_allow_methods(),
        }
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            log_format: default_log_format(),
            log_level: default_log_level(),
            request_id_header: default_request_id_header(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            header_wallet_address: default_header_wallet_address(),
            header_wallet_signature: default_header_wallet_signature(),
            header_wallet_nonce: default_header_wallet_nonce(),
            nonce_ttl_secs: default_nonce_ttl_secs(),
            redis_key_prefix: default_redis_key_prefix(),
            bypass_paths: default_bypass_paths(),
            protect_prefixes: default_protect_prefixes(),
            require_https: false,
            accept_signature_b58: true,
            accept_signature_b64: false,
            canonicalize_method: default_canonicalize_method(),
            canonicalize_path: default_canonicalize_path(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: default_rate_limit_strategy(),
            respect_x_forwarded_for: true,
            ip_max_requests: default_ip_max_requests(),
            ip_window_secs: default_ip_window_secs(),
            user_max_requests: default_user_max_requests(),
            user_window_secs: default_user_window_secs(),
        }
    }
}

impl Default for IntegrationsConfig {
    fn default() -> Self {
        Self {
            enable_postgres: true,
            enable_redis: true,
            enable_kafka: true,
            database_url: String::new(),
            pg_max_connections: default_pg_max_connections(),
            pg_connect_timeout_ms: default_pg_connect_timeout_ms(),
            pg_idle_timeout_ms: default_pg_idle_timeout_ms(),
            redis_url: String::new(),
            redis_connect_timeout_ms: default_redis_connect_timeout_ms(),
            redis_command_timeout_ms: default_redis_command_timeout_ms(),
            kafka_brokers: String::new(),
            kafka_client_id: default_kafka_client_id(),
            kafka_metadata_timeout_ms: default_kafka_metadata_timeout_ms(),
        }
    }
}

fn default_cache_backend() -> String {
    "memory".to_string()
}

fn default_cache_ttl_secs() -> u64 {
    10
}

fn default_cache_max_entries() -> usize {
    1000
}

fn default_etag_salt() -> String {
    String::new()
}

#[derive(Debug, Deserialize, Clone)]
pub struct CacheConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_cache_backend")]
    pub backend: String, // "memory" | "redis"
    #[serde(default = "default_cache_ttl_secs")]
    pub ttl_secs: u64,
    #[serde(default = "default_cache_max_entries")]
    pub max_entries: usize,
    #[serde(default = "default_etag_salt")]
    pub etag_salt: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: default_cache_backend(),
            ttl_secs: default_cache_ttl_secs(),
            max_entries: default_cache_max_entries(),
            etag_salt: default_etag_salt(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct WsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_ws_path")]
    pub path: String,
    #[serde(default = "default_ping_interval_secs")]
    pub ping_interval_secs: u64,
    #[serde(default = "default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
    #[serde(default = "default_max_subscriptions_per_conn")]
    pub max_subscriptions_per_conn: u32,
    #[serde(default = "default_max_client_msg_per_min")]
    pub max_client_msg_per_min: u32,
    #[serde(default = "default_max_events_per_sec")]
    pub max_events_per_sec: u32,
    #[serde(default = "default_ws_source")]
    pub source: String, // "poll" | "redis"
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
    #[serde(default = "default_redis_channel")]
    pub redis_channel: String,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: default_ws_path(),
            ping_interval_secs: default_ping_interval_secs(),
            idle_timeout_secs: default_idle_timeout_secs(),
            max_subscriptions_per_conn: default_max_subscriptions_per_conn(),
            max_client_msg_per_min: default_max_client_msg_per_min(),
            max_events_per_sec: default_max_events_per_sec(),
            source: default_ws_source(),
            poll_interval_ms: default_poll_interval_ms(),
            redis_channel: default_redis_channel(),
        }
    }
}

fn default_ws_path() -> String {
    "/ws/tx".to_string()
}

fn default_ping_interval_secs() -> u64 {
    20
}

fn default_idle_timeout_secs() -> u64 {
    60
}

fn default_max_subscriptions_per_conn() -> u32 {
    10
}

fn default_max_client_msg_per_min() -> u32 {
    30
}

fn default_max_events_per_sec() -> u32 {
    100
}

fn default_ws_source() -> String {
    "poll".to_string()
}

fn default_poll_interval_ms() -> u64 {
    500
}

fn default_redis_channel() -> String {
    "tx:new".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct KafkaConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_kafka_brokers")]
    pub brokers: String,
    #[serde(default = "default_kafka_group_id")]
    pub group_id: String,
    #[serde(default = "default_kafka_input_topic")]
    pub input_topic: String,
    #[serde(default = "default_kafka_dlq_topic")]
    pub dlq_topic: String,
    #[serde(default)]
    pub enable_auto_commit: bool,
    #[serde(default = "default_kafka_max_poll_records")]
    pub max_poll_records: i32,
    #[serde(default = "default_kafka_poll_interval_ms")]
    pub poll_interval_ms: u64,
    #[serde(default = "default_kafka_session_timeout_ms")]
    pub session_timeout_ms: u64,
    #[serde(default = "default_kafka_message_max_bytes")]
    pub message_max_bytes: i32,
    #[serde(default = "default_kafka_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
    #[serde(default = "default_kafka_max_retries")]
    pub max_retries: u32,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            brokers: default_kafka_brokers(),
            group_id: default_kafka_group_id(),
            input_topic: default_kafka_input_topic(),
            dlq_topic: default_kafka_dlq_topic(),
            enable_auto_commit: false,
            max_poll_records: default_kafka_max_poll_records(),
            poll_interval_ms: default_kafka_poll_interval_ms(),
            session_timeout_ms: default_kafka_session_timeout_ms(),
            message_max_bytes: default_kafka_message_max_bytes(),
            retry_backoff_ms: default_kafka_retry_backoff_ms(),
            max_retries: default_kafka_max_retries(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct IngestConfig {
    #[serde(default = "default_max_inflight_batches")]
    pub max_inflight_batches: u32,
    #[serde(default = "default_db_insert_batch_size")]
    pub db_insert_batch_size: usize,
    #[serde(default = "default_true")]
    pub emit_ws_events: bool,
    #[serde(default = "default_true")]
    pub idempotency_by_signature: bool,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            max_inflight_batches: default_max_inflight_batches(),
            db_insert_batch_size: default_db_insert_batch_size(),
            emit_ws_events: true,
            idempotency_by_signature: true,
        }
    }
}

fn default_kafka_brokers() -> String {
    "127.0.0.1:9092".to_string()
}

fn default_kafka_group_id() -> String {
    "blockchain-api-consumer".to_string()
}

fn default_kafka_input_topic() -> String {
    "tx.raw".to_string()
}

fn default_kafka_dlq_topic() -> String {
    "tx.dlq".to_string()
}

fn default_kafka_max_poll_records() -> i32 {
    100
}

fn default_kafka_poll_interval_ms() -> u64 {
    200
}

fn default_kafka_session_timeout_ms() -> u64 {
    10000
}

fn default_kafka_message_max_bytes() -> i32 {
    1048576 // 1 MiB
}

fn default_kafka_retry_backoff_ms() -> u64 {
    200
}

fn default_kafka_max_retries() -> u32 {
    5
}

fn default_max_inflight_batches() -> u32 {
    4
}

fn default_db_insert_batch_size() -> usize {
    100
}

#[derive(Debug, Deserialize, Clone)]
pub struct WafConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_waf_mode")]
    pub mode: String, // "shadow" | "block"
    #[serde(default = "default_true")]
    pub respect_x_forwarded_for: bool,
    #[serde(default = "default_bypass_paths")]
    pub bypass_paths: Vec<String>,
    #[serde(default = "default_max_request_body_bytes")]
    pub max_request_body_bytes: usize,
    #[serde(default = "default_max_query_length")]
    pub max_query_length: usize,
    #[serde(default = "default_allowed_methods")]
    pub allowed_methods: Vec<String>,
    #[serde(default = "default_true")]
    pub use_redis_lists: bool,
    #[serde(default = "default_redis_ban_set")]
    pub redis_ban_set: String,
    #[serde(default = "default_redis_grey_set")]
    pub redis_grey_set: String,
    #[serde(default = "default_ban_ttl_secs")]
    pub ban_ttl_secs: u64,
    #[serde(default = "default_grey_ttl_secs")]
    pub grey_ttl_secs: u64,
    #[serde(default = "default_blocked_ua_substrings")]
    pub blocked_ua_substrings: Vec<String>,
    #[serde(default = "default_blocked_path_patterns")]
    pub blocked_path_patterns: Vec<String>,
    #[serde(default = "default_sqli_patterns")]
    pub sqli_patterns: Vec<String>,
    #[serde(default = "default_xss_patterns")]
    pub xss_patterns: Vec<String>,
    #[serde(default = "default_rce_patterns")]
    pub rce_patterns: Vec<String>,
    #[serde(default = "default_path_traversal_patterns")]
    pub path_traversal_patterns: Vec<String>,
    #[serde(default = "default_score_weights")]
    pub score_weights: std::collections::HashMap<String, u32>,
    #[serde(default = "default_block_threshold")]
    pub block_threshold: u32,
    #[serde(default = "default_grey_threshold")]
    pub grey_threshold: u32,
    #[serde(default = "default_max_events_per_ip_per_min")]
    pub max_events_per_ip_per_min: u32,
}

impl Default for WafConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: default_waf_mode(),
            respect_x_forwarded_for: true,
            bypass_paths: default_bypass_paths(),
            max_request_body_bytes: default_max_request_body_bytes(),
            max_query_length: default_max_query_length(),
            allowed_methods: default_allowed_methods(),
            use_redis_lists: true,
            redis_ban_set: default_redis_ban_set(),
            redis_grey_set: default_redis_grey_set(),
            ban_ttl_secs: default_ban_ttl_secs(),
            grey_ttl_secs: default_grey_ttl_secs(),
            blocked_ua_substrings: default_blocked_ua_substrings(),
            blocked_path_patterns: default_blocked_path_patterns(),
            sqli_patterns: default_sqli_patterns(),
            xss_patterns: default_xss_patterns(),
            rce_patterns: default_rce_patterns(),
            path_traversal_patterns: default_path_traversal_patterns(),
            score_weights: default_score_weights(),
            block_threshold: default_block_threshold(),
            grey_threshold: default_grey_threshold(),
            max_events_per_ip_per_min: default_max_events_per_ip_per_min(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminConfig {
    #[serde(default = "default_true")]
    pub enable_debug_route: bool,
    #[serde(default = "default_debug_route_path")]
    pub debug_route_path: String,
    #[serde(default = "default_admin_header")]
    pub admin_header: String,
    #[serde(default = "default_admin_token")]
    pub admin_token: String,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enable_debug_route: true,
            debug_route_path: default_debug_route_path(),
            admin_header: default_admin_header(),
            admin_token: default_admin_token(),
        }
    }
}

fn default_waf_mode() -> String {
    "shadow".to_string()
}


fn default_max_request_body_bytes() -> usize {
    1048576 // 1 MiB
}

fn default_max_query_length() -> usize {
    4096
}

fn default_allowed_methods() -> Vec<String> {
    vec![
        "GET".to_string(),
        "POST".to_string(),
        "PUT".to_string(),
        "PATCH".to_string(),
        "DELETE".to_string(),
        "OPTIONS".to_string(),
    ]
}

fn default_redis_ban_set() -> String {
    "waf:ban:ips".to_string()
}

fn default_redis_grey_set() -> String {
    "waf:grey:ips".to_string()
}

fn default_ban_ttl_secs() -> u64 {
    3600
}

fn default_grey_ttl_secs() -> u64 {
    300
}

fn default_blocked_ua_substrings() -> Vec<String> {
    vec![
        "sqlmap".to_string(),
        "acunetix".to_string(),
        "nmap".to_string(),
        "dirbuster".to_string(),
    ]
}

fn default_blocked_path_patterns() -> Vec<String> {
    vec![
        r"(?i)\.(?:env|git|svn)(?:$|/)".to_string(),
        r"(?i)\bwp-admin\b".to_string(),
        r"(?i)\bphpmyadmin\b".to_string(),
    ]
}

fn default_sqli_patterns() -> Vec<String> {
    vec![
        r"(?i)\bUNION\b\s+\bSELECT\b".to_string(),
        r"(?i)\bOR\b\s+1=1\b".to_string(),
        r"(?i)\bSLEEP\s*\(".to_string(),
    ]
}

fn default_xss_patterns() -> Vec<String> {
    vec![
        r"(?i)<\s*script\b".to_string(),
        r"(?i)onerror\s*=".to_string(),
        r"(?i)javascript:".to_string(),
    ]
}

fn default_rce_patterns() -> Vec<String> {
    vec![
        r"(?i)\b(?:/bin/sh|/bin/bash)\b".to_string(),
        r"(?i)\b\|\s*\b(?:cat|ls|curl|wget)\b".to_string(),
    ]
}

fn default_path_traversal_patterns() -> Vec<String> {
    vec![
        r"\.\./".to_string(),
        r"%2e%2e/".to_string(),
    ]
}

fn default_score_weights() -> std::collections::HashMap<String, u32> {
    let mut weights = std::collections::HashMap::new();
    weights.insert("sqli".to_string(), 8);
    weights.insert("xss".to_string(), 6);
    weights.insert("rce".to_string(), 8);
    weights.insert("traversal".to_string(), 6);
    weights.insert("bad_ua".to_string(), 4);
    weights.insert("bad_path".to_string(), 4);
    weights.insert("oversize".to_string(), 5);
    weights
}

fn default_block_threshold() -> u32 {
    10
}

fn default_grey_threshold() -> u32 {
    6
}

fn default_max_events_per_ip_per_min() -> u32 {
    60
}

fn default_debug_route_path() -> String {
    "/_waf/debug".to_string()
}

fn default_admin_header() -> String {
    "X-Admin-Token".to_string()
}

fn default_admin_token() -> String {
    "".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct OtelConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_service_name")]
    pub service_name: String,
    #[serde(default = "default_service_namespace")]
    pub service_namespace: String,
    #[serde(default = "default_service_version")]
    pub service_version: String,
    #[serde(default = "default_resource_attributes")]
    pub resource_attributes: String,
    #[serde(default)]
    pub traces: OtelTracesConfig,
    #[serde(default)]
    pub metrics: OtelMetricsConfig,
    #[serde(default)]
    pub logs: OtelLogsConfig,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            service_name: default_service_name(),
            service_namespace: default_service_namespace(),
            service_version: default_service_version(),
            resource_attributes: default_resource_attributes(),
            traces: OtelTracesConfig::default(),
            metrics: OtelMetricsConfig::default(),
            logs: OtelLogsConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct OtelTracesConfig {
    #[serde(default = "default_traces_exporter")]
    pub exporter: String,
    #[serde(default = "default_otlp_endpoint")]
    pub otlp_endpoint: String,
    #[serde(default = "default_traces_protocol")]
    pub protocol: String,
    #[serde(default = "default_sample_ratio")]
    pub sample_ratio: f64,
    #[serde(default = "default_true")]
    pub include_internal: bool,
}

impl Default for OtelTracesConfig {
    fn default() -> Self {
        Self {
            exporter: default_traces_exporter(),
            otlp_endpoint: default_otlp_endpoint(),
            protocol: default_traces_protocol(),
            sample_ratio: default_sample_ratio(),
            include_internal: true,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct OtelMetricsConfig {
    #[serde(default = "default_metrics_exporter")]
    pub exporter: String,
    #[serde(default = "default_prometheus_bind")]
    pub prometheus_bind: String,
    #[serde(default = "default_otlp_endpoint")]
    pub otlp_endpoint: String,
}

impl Default for OtelMetricsConfig {
    fn default() -> Self {
        Self {
            exporter: default_metrics_exporter(),
            prometheus_bind: default_prometheus_bind(),
            otlp_endpoint: default_otlp_endpoint(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct OtelLogsConfig {
    #[serde(default = "default_true")]
    pub inject_trace_ids: bool,
}

impl Default for OtelLogsConfig {
    fn default() -> Self {
        Self {
            inject_trace_ids: true,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct SentryConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_sentry_dsn")]
    pub dsn: String,
    #[serde(default = "default_sentry_environment")]
    pub environment: String,
    #[serde(default = "default_sentry_release")]
    pub release: String,
    #[serde(default = "default_sentry_traces_sample_rate")]
    pub traces_sample_rate: f64,
}

impl Default for SentryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            dsn: default_sentry_dsn(),
            environment: default_sentry_environment(),
            release: default_sentry_release(),
            traces_sample_rate: default_sentry_traces_sample_rate(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MetricsConfig {
    #[serde(default = "default_http_request_labels")]
    pub http_request_labels: Vec<String>,
    #[serde(default = "default_ws_labels")]
    pub ws_labels: Vec<String>,
    #[serde(default = "default_cache_labels")]
    pub cache_labels: Vec<String>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            http_request_labels: default_http_request_labels(),
            ws_labels: default_ws_labels(),
            cache_labels: default_cache_labels(),
        }
    }
}

// Default functions for new configs
fn default_service_namespace() -> String {
    "pumpnex".to_string()
}


fn default_resource_attributes() -> String {
    "deployment.environment=dev".to_string()
}

fn default_traces_exporter() -> String {
    "otlp".to_string()
}

fn default_otlp_endpoint() -> String {
    "http://127.0.0.1:4317".to_string()
}

fn default_traces_protocol() -> String {
    "grpc".to_string()
}

fn default_sample_ratio() -> f64 {
    0.1
}

fn default_metrics_exporter() -> String {
    "prometheus".to_string()
}

fn default_prometheus_bind() -> String {
    "0.0.0.0:9464".to_string()
}

fn default_sentry_dsn() -> String {
    "".to_string()
}

fn default_sentry_environment() -> String {
    "dev".to_string()
}

fn default_sentry_release() -> String {
    "blockchain-api@0.2.0".to_string()
}

fn default_sentry_traces_sample_rate() -> f64 {
    0.0
}

fn default_http_request_labels() -> Vec<String> {
    vec!["method".to_string(), "path".to_string(), "status".to_string()]
}

fn default_ws_labels() -> Vec<String> {
    vec!["event".to_string(), "ok".to_string()]
}

fn default_cache_labels() -> Vec<String> {
    vec!["backend".to_string(), "op".to_string(), "hit".to_string()]
}

// New configuration structures for TT-12

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    #[serde(default = "default_cors_allowed_origins")]
    pub cors_allowed_origins: Vec<String>,
    #[serde(default = "default_cors_allowed_methods")]
    pub cors_allowed_methods: Vec<String>,
    #[serde(default = "default_cors_allowed_headers")]
    pub cors_allowed_headers: Vec<String>,
    #[serde(default = "default_false")]
    pub hsts_enabled: bool,
    #[serde(default = "default_hsts_max_age_secs")]
    pub hsts_max_age_secs: u64,
    #[serde(default = "default_frame_options")]
    pub frame_options: String,
    #[serde(default = "default_referrer_policy")]
    pub referrer_policy: String,
    #[serde(default = "default_x_content_type_options")]
    pub x_content_type_options: String,
    #[serde(default = "default_permissions_policy")]
    pub permissions_policy: String,
    #[serde(default = "default_false")]
    pub csp_enabled: bool,
    #[serde(default = "default_csp")]
    pub csp: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TlsConfig {
    #[serde(default = "default_false")]
    pub enabled: bool,
    #[serde(default = "default_cert_path")]
    pub cert_path: String,
    #[serde(default = "default_key_path")]
    pub key_path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DbConfig {
    #[serde(default = "default_false")]
    pub run_migrations_on_start: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeployConfig {
    #[serde(default = "default_service_http_port")]
    pub service_http_port: u16,
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
    #[serde(default = "default_graceful_shutdown_secs")]
    pub graceful_shutdown_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ImageConfig {
    #[serde(default = "default_image_name")]
    pub name: String,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            cors_allowed_origins: default_cors_allowed_origins(),
            cors_allowed_methods: default_cors_allowed_methods(),
            cors_allowed_headers: default_cors_allowed_headers(),
            hsts_enabled: false,
            hsts_max_age_secs: default_hsts_max_age_secs(),
            frame_options: default_frame_options(),
            referrer_policy: default_referrer_policy(),
            x_content_type_options: default_x_content_type_options(),
            permissions_policy: default_permissions_policy(),
            csp_enabled: false,
            csp: default_csp(),
        }
    }
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cert_path: default_cert_path(),
            key_path: default_key_path(),
        }
    }
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            run_migrations_on_start: false,
        }
    }
}

impl Default for DeployConfig {
    fn default() -> Self {
        Self {
            service_http_port: default_service_http_port(),
            metrics_port: default_metrics_port(),
            graceful_shutdown_secs: default_graceful_shutdown_secs(),
        }
    }
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            name: default_image_name(),
        }
    }
}

// Default functions for new configs
fn default_cors_allowed_origins() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_cors_allowed_methods() -> Vec<String> {
    vec![
        "GET".to_string(),
        "POST".to_string(),
        "PUT".to_string(),
        "PATCH".to_string(),
        "DELETE".to_string(),
        "OPTIONS".to_string(),
    ]
}

fn default_cors_allowed_headers() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_hsts_max_age_secs() -> u64 {
    31536000
}

fn default_frame_options() -> String {
    "DENY".to_string()
}

fn default_referrer_policy() -> String {
    "no-referrer".to_string()
}

fn default_x_content_type_options() -> String {
    "nosniff".to_string()
}

fn default_permissions_policy() -> String {
    "geolocation=(), microphone=(), camera=()".to_string()
}

fn default_csp() -> String {
    "default-src 'none'; frame-ancestors 'none';".to_string()
}

fn default_cert_path() -> String {
    "/etc/blockchain-api/tls/cert.pem".to_string()
}

fn default_key_path() -> String {
    "/etc/blockchain-api/tls/key.pem".to_string()
}

fn default_service_http_port() -> u16 {
    8080
}

fn default_metrics_port() -> u16 {
    9464
}

fn default_graceful_shutdown_secs() -> u64 {
    10
}

fn default_image_name() -> String {
    "ghcr.io/OWNER/blockchain-api".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct RuntimeConfig {
    #[serde(default = "default_worker_threads")]
    pub worker_threads: u32,
    #[serde(default = "default_max_blocking_threads")]
    pub max_blocking_threads: u32,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            worker_threads: default_worker_threads(),
            max_blocking_threads: default_max_blocking_threads(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoadConfig {
    #[serde(default = "default_load_base_url")]
    pub default_base_url: String,
}

impl Default for LoadConfig {
    fn default() -> Self {
        LoadConfig {
            default_base_url: default_load_base_url(),
        }
    }
}

fn default_worker_threads() -> u32 {
    0
}

fn default_max_blocking_threads() -> u32 {
    512
}

fn default_load_base_url() -> String {
    "http://127.0.0.1:8080".to_string()
}

fn default_workers() -> u32 {
    0
}


pub fn load_config() -> Result<Config, config::ConfigError> {
    let env = env::var("APP__ENV").unwrap_or_else(|_| "dev".to_string());

    let mut builder = config::Config::builder();

    // Try to load TOML file, but don't fail if it doesn't exist
    let config_path = format!("configs/{}/default", env);
    if std::path::Path::new(&format!("{}.toml", config_path)).exists() {
        builder = builder.add_source(config::File::with_name(&config_path).required(false));
    }

    // Environment variables override with APP__ prefix
    builder = builder.add_source(
        config::Environment::with_prefix("APP")
            .separator("__")
            .try_parsing(true),
    );

    let config = builder.build()?;
    config.try_deserialize()
}
