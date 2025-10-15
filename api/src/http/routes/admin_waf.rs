/// Admin WAF debug routes
///
/// Provides diagnostic information about WAF configuration and recent events
/// for debugging and monitoring purposes.

use crate::{
    app_state::AppState,
    config::{AdminConfig, WafConfig},
    infra::redis,
};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde_json::json;
use std::collections::HashMap;

/// WAF debug information
#[derive(Debug, serde::Serialize)]
pub struct WafDebugInfo {
    pub config: WafDebugConfig,
    pub stats: WafStats,
    pub recent_events: Vec<WafEvent>,
}

#[derive(Debug, serde::Serialize)]
pub struct WafDebugConfig {
    pub enabled: bool,
    pub mode: String,
    pub bypass_paths: Vec<String>,
    pub max_request_body_bytes: usize,
    pub max_query_length: usize,
    pub allowed_methods: Vec<String>,
    pub use_redis_lists: bool,
    pub redis_ban_set: String,
    pub redis_grey_set: String,
    pub ban_ttl_secs: u64,
    pub grey_ttl_secs: u64,
    pub block_threshold: u32,
    pub grey_threshold: u32,
    pub max_events_per_ip_per_min: u32,
    pub pattern_counts: HashMap<String, usize>,
}

#[derive(Debug, serde::Serialize)]
pub struct WafStats {
    pub total_requests: u64,
    pub blocked_requests: u64,
    pub grey_requests: u64,
    pub passed_requests: u64,
    pub ban_list_size: u64,
    pub grey_list_size: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct WafEvent {
    pub timestamp: String,
    pub ip: String,
    pub method: String,
    pub path: String,
    pub score: u32,
    pub action: String,
    pub matches: Vec<String>,
}

/// Get WAF debug information
pub async fn waf_debug(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    waf_config: web::Data<WafConfig>,
    admin_config: web::Data<AdminConfig>,
) -> impl Responder {
    // Check admin authentication if token is configured
    if !admin_config.admin_token.is_empty() {
        if let Some(token) = req.headers().get(&admin_config.admin_header) {
            if let Ok(token_str) = token.to_str() {
                if token_str != admin_config.admin_token {
                    return HttpResponse::Forbidden().json(json!({
                        "error": "forbidden",
                        "message": "Invalid admin token"
                    }));
                }
            } else {
                return HttpResponse::Forbidden().json(json!({
                    "error": "forbidden",
                    "message": "Invalid admin token format"
                }));
            }
        } else {
            return HttpResponse::Forbidden().json(json!({
                "error": "forbidden",
                "message": "Admin token required"
            }));
        }
    }

    // Get WAF configuration (without sensitive data)
    let config = WafDebugConfig {
        enabled: waf_config.enabled,
        mode: waf_config.mode.clone(),
        bypass_paths: waf_config.bypass_paths.clone(),
        max_request_body_bytes: waf_config.max_request_body_bytes,
        max_query_length: waf_config.max_query_length,
        allowed_methods: waf_config.allowed_methods.clone(),
        use_redis_lists: waf_config.use_redis_lists,
        redis_ban_set: waf_config.redis_ban_set.clone(),
        redis_grey_set: waf_config.redis_grey_set.clone(),
        ban_ttl_secs: waf_config.ban_ttl_secs,
        grey_ttl_secs: waf_config.grey_ttl_secs,
        block_threshold: waf_config.block_threshold,
        grey_threshold: waf_config.grey_threshold,
        max_events_per_ip_per_min: waf_config.max_events_per_ip_per_min,
        pattern_counts: get_pattern_counts(&waf_config),
    };

    // Get statistics
    let stats = get_waf_stats(&app_state, &waf_config).await;

    // Get recent events (placeholder - would come from actual event storage)
    let recent_events = get_recent_events().await;

    let debug_info = WafDebugInfo {
        config,
        stats,
        recent_events,
    };

    HttpResponse::Ok().json(debug_info)
}

/// Get pattern counts for each category
fn get_pattern_counts(waf_config: &WafConfig) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    counts.insert("blocked_paths".to_string(), waf_config.blocked_path_patterns.len());
    counts.insert("sqli".to_string(), waf_config.sqli_patterns.len());
    counts.insert("xss".to_string(), waf_config.xss_patterns.len());
    counts.insert("rce".to_string(), waf_config.rce_patterns.len());
    counts.insert("path_traversal".to_string(), waf_config.path_traversal_patterns.len());
    counts.insert("blocked_ua".to_string(), waf_config.blocked_ua_substrings.len());
    counts
}

/// Get WAF statistics
async fn get_waf_stats(app_state: &AppState, waf_config: &WafConfig) -> WafStats {
    let mut ban_list_size = 0;
    let mut grey_list_size = 0;

    // Get Redis list sizes if Redis is available
    if waf_config.use_redis_lists {
        if let Some(redis_manager) = &app_state.redis {
            // Get ban list size
            if let Ok(size) = redis::get_set_size(&mut redis_manager.clone(), &waf_config.redis_ban_set).await {
                ban_list_size = size;
            }

            // Get grey list size
            if let Ok(size) = redis::get_set_size(&mut redis_manager.clone(), &waf_config.redis_grey_set).await {
                grey_list_size = size;
            }
        }
    }

    WafStats {
        total_requests: 0, // Would come from actual metrics
        blocked_requests: 0,
        grey_requests: 0,
        passed_requests: 0,
        ban_list_size,
        grey_list_size,
    }
}

/// Get recent WAF events (placeholder implementation)
async fn get_recent_events() -> Vec<WafEvent> {
    // In a real implementation, this would query from a log store or metrics system
    vec![
        WafEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            ip: "127.0.0.1".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            score: 0,
            action: "pass".to_string(),
            matches: vec![],
        },
    ]
}
