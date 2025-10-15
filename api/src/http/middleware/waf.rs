/// WAF (Web Application Firewall) middleware
///
/// Provides request inspection, anomaly detection, and ban/grey list management
/// with pattern matching for common attack vectors.

use crate::{
    app_state::AppState,
    config::WafConfig,
    infra::redis,
};
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::StatusCode,
    Error, HttpRequest, HttpResponse,
};
use regex::RegexSet;
use serde_json::json;
use std::{
    collections::HashMap,
    future::{ready, Ready},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{error, info, warn};

/// WAF middleware state
#[derive(Clone)]
pub struct WafMiddleware {
    config: WafConfig,
    patterns: Arc<WafPatterns>,
    event_counts: Arc<std::sync::Mutex<HashMap<String, (u32, Instant)>>>,
    app_state: Option<Arc<AppState>>,
}

/// Precompiled regex patterns for efficient matching
#[derive(Debug)]
pub struct WafPatterns {
    pub blocked_paths: RegexSet,
    pub sqli: RegexSet,
    pub xss: RegexSet,
    pub rce: RegexSet,
    pub path_traversal: RegexSet,
}

/// WAF analysis result
#[derive(Debug, Clone)]
pub struct WafResult {
    pub score: u32,
    pub matches: Vec<WafMatch>,
    pub action: WafAction,
    pub client_ip: String,
}

/// Individual pattern match
#[derive(Debug, Clone)]
pub struct WafMatch {
    pub category: String,
    pub pattern: String,
    pub weight: u32,
}

/// WAF action to take
#[derive(Debug, Clone)]
pub enum WafAction {
    Pass,
    Grey,
    Block,
}

impl WafMiddleware {
    /// Create new WAF middleware
    pub fn new(config: WafConfig, app_state: Option<Arc<AppState>>) -> Result<Self, String> {
        let patterns = Arc::new(Self::compile_patterns(&config)?);
        let event_counts = Arc::new(std::sync::Mutex::new(HashMap::new()));

        Ok(Self {
            config,
            patterns,
            event_counts,
            app_state,
        })
    }

    /// Compile regex patterns for efficient matching
    fn compile_patterns(config: &WafConfig) -> Result<WafPatterns, String> {
        let blocked_paths = RegexSet::new(&config.blocked_path_patterns)
            .map_err(|e| format!("Invalid blocked path patterns: {}", e))?;

        let sqli = RegexSet::new(&config.sqli_patterns)
            .map_err(|e| format!("Invalid SQLi patterns: {}", e))?;

        let xss = RegexSet::new(&config.xss_patterns)
            .map_err(|e| format!("Invalid XSS patterns: {}", e))?;

        let rce = RegexSet::new(&config.rce_patterns)
            .map_err(|e| format!("Invalid RCE patterns: {}", e))?;

        let path_traversal = RegexSet::new(&config.path_traversal_patterns)
            .map_err(|e| format!("Invalid path traversal patterns: {}", e))?;

        Ok(WafPatterns {
            blocked_paths,
            sqli,
            xss,
            rce,
            path_traversal,
        })
    }

    /// Extract client IP from request
    fn extract_client_ip(&self, req: &HttpRequest) -> String {
        if self.config.respect_x_forwarded_for {
            if let Some(forwarded) = req.headers().get("X-Forwarded-For") {
                if let Ok(forwarded_str) = forwarded.to_str() {
                    // Take the first IP from the chain
                    if let Some(first_ip) = forwarded_str.split(',').next() {
                        let ip = first_ip.trim();
                        if self.is_valid_ip(ip) {
                            return ip.to_string();
                        }
                    }
                }
            }
        }

        // Fallback to peer address
        if let Some(peer_addr) = req.peer_addr() {
            peer_addr.ip().to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Validate IP address format
    fn is_valid_ip(&self, ip: &str) -> bool {
        ip.parse::<std::net::IpAddr>().is_ok()
    }

    /// Check if path should be bypassed
    fn is_bypassed(&self, path: &str) -> bool {
        self.config.bypass_paths.iter().any(|bypass| {
            path.starts_with(bypass) || path == bypass
        })
    }

    /// Check if method is allowed
    fn is_method_allowed(&self, method: &str) -> bool {
        self.config.allowed_methods.contains(&method.to_string())
    }

    /// Analyze request for anomalies
    fn analyze_request(&self, req: &HttpRequest) -> WafResult {
        let client_ip = self.extract_client_ip(req);
        let method = req.method().as_str();
        let path = req.path();
        let query = req.query_string();
        let user_agent = req
            .headers()
            .get("User-Agent")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        let mut score = 0u32;
        let mut matches = Vec::new();

        // Check if IP is in ban list (would be checked separately)
        if self.is_banned(&client_ip) {
            return WafResult {
                score: 999, // High score for banned IPs
                matches: vec![WafMatch {
                    category: "banned".to_string(),
                    pattern: "banned_ip".to_string(),
                    weight: 999,
                }],
                action: WafAction::Block,
                client_ip,
            };
        }

        // Check if IP is in grey list
        let is_grey = self.is_grey(&client_ip);
        if is_grey {
            score += 2; // Start with +2 for grey IPs
        }

        // Check method
        if !self.is_method_allowed(method) {
            score += 3;
            matches.push(WafMatch {
                category: "bad_method".to_string(),
                pattern: method.to_string(),
                weight: 3,
            });
        }

        // Check query length
        if query.len() > self.config.max_query_length {
            score += self.config.score_weights.get("oversize").copied().unwrap_or(5);
            matches.push(WafMatch {
                category: "oversize".to_string(),
                pattern: "query_too_long".to_string(),
                weight: self.config.score_weights.get("oversize").copied().unwrap_or(5),
            });
        }

        // Check User-Agent
        for blocked_ua in &self.config.blocked_ua_substrings {
            if user_agent.to_lowercase().contains(&blocked_ua.to_lowercase()) {
                let weight = self.config.score_weights.get("bad_ua").copied().unwrap_or(4);
                score += weight;
                matches.push(WafMatch {
                    category: "bad_ua".to_string(),
                    pattern: blocked_ua.clone(),
                    weight,
                });
            }
        }

        // Check path patterns
        if self.patterns.blocked_paths.is_match(path) {
            let weight = self.config.score_weights.get("bad_path").copied().unwrap_or(4);
            score += weight;
            matches.push(WafMatch {
                category: "bad_path".to_string(),
                pattern: "blocked_path".to_string(),
                weight,
            });
        }

        // Check for SQLi patterns
        let text_to_check = format!("{} {}", path, query);
        if self.patterns.sqli.is_match(&text_to_check) {
            let weight = self.config.score_weights.get("sqli").copied().unwrap_or(8);
            score += weight;
            matches.push(WafMatch {
                category: "sqli".to_string(),
                pattern: "sqli_detected".to_string(),
                weight,
            });
        }

        // Check for XSS patterns
        if self.patterns.xss.is_match(&text_to_check) {
            let weight = self.config.score_weights.get("xss").copied().unwrap_or(6);
            score += weight;
            matches.push(WafMatch {
                category: "xss".to_string(),
                pattern: "xss_detected".to_string(),
                weight,
            });
        }

        // Check for RCE patterns
        if self.patterns.rce.is_match(&text_to_check) {
            let weight = self.config.score_weights.get("rce").copied().unwrap_or(8);
            score += weight;
            matches.push(WafMatch {
                category: "rce".to_string(),
                pattern: "rce_detected".to_string(),
                weight,
            });
        }

        // Check for path traversal
        if self.patterns.path_traversal.is_match(&text_to_check) {
            let weight = self.config.score_weights.get("traversal").copied().unwrap_or(6);
            score += weight;
            matches.push(WafMatch {
                category: "traversal".to_string(),
                pattern: "path_traversal_detected".to_string(),
                weight,
            });
        }

        // Determine action based on score
        let action = if score >= self.config.block_threshold {
            WafAction::Block
        } else if score >= self.config.grey_threshold {
            WafAction::Grey
        } else {
            WafAction::Pass
        };

        WafResult {
            score,
            matches,
            action,
            client_ip,
        }
    }

    /// Check if IP is banned
    fn is_banned(&self, _ip: &str) -> bool {
        // Temporarily disabled to avoid runtime conflicts
        // TODO: Implement proper async Redis checks
        false
    }

    /// Check if IP is in grey list
    fn is_grey(&self, _ip: &str) -> bool {
        // Temporarily disabled to avoid runtime conflicts
        // TODO: Implement proper async Redis checks
        false
    }

    /// Add IP to ban list
    fn add_to_ban(&self, _ip: &str) {
        // Temporarily disabled to avoid runtime conflicts
        // TODO: Implement proper async Redis operations
        info!("Would add IP to ban list (disabled)");
    }

    /// Add IP to grey list
    fn add_to_grey(&self, _ip: &str) {
        // Temporarily disabled to avoid runtime conflicts
        // TODO: Implement proper async Redis operations
        info!("Would add IP to grey list (disabled)");
    }

    /// Log WAF event
    fn log_event(&self, result: &WafResult, req: &HttpRequest) {
        let method = req.method().as_str();
        let path = req.path();
        let user_agent = req
            .headers()
            .get("User-Agent")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        let action_str = match result.action {
            WafAction::Pass => "pass",
            WafAction::Grey => "grey",
            WafAction::Block => "block",
        };

        info!(
            ip = %result.client_ip,
            method = %method,
            path = %path,
            ua_hash = %format!("{:x}", md5::compute(user_agent.as_bytes())),
            score = %result.score,
            matches = ?result.matches,
            mode = %self.config.mode,
            action = %action_str,
            "WAF event"
        );
    }

    /// Check rate limit for events per IP
    fn check_event_rate_limit(&self, ip: &str) -> bool {
        let mut counts = self.event_counts.lock().unwrap();
        let now = Instant::now();
        let window = Duration::from_secs(60);

        if let Some((count, last_reset)) = counts.get(ip) {
            if now.duration_since(*last_reset) < window {
                if *count >= self.config.max_events_per_ip_per_min {
                    return false; // Rate limited
                }
                *counts.get_mut(ip).unwrap() = (*count + 1, *last_reset);
            } else {
                counts.insert(ip.to_string(), (1, now));
            }
        } else {
            counts.insert(ip.to_string(), (1, now));
        }

        true
    }
}

impl<S, B> Transform<S, ServiceRequest> for WafMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = WafService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(WafService {
            service: Rc::new(service),
            waf: self.clone(),
        }))
    }
}

#[derive(Clone)]
pub struct WafService<S> {
    service: Rc<S>,
    waf: WafMiddleware,
}

impl<S, B> Service<ServiceRequest> for WafService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let waf = self.waf.clone();

        Box::pin(async move {
            // Skip WAF if disabled
            if !waf.config.enabled {
                let res = service.call(req).await?;
                return Ok(res.map_into_left_body());
            }

            let path = req.path();
            let method = req.method().as_str();

            // Check bypass paths
            if waf.is_bypassed(path) {
                let res = service.call(req).await?;
                return Ok(res.map_into_left_body());
            }

            // Analyze request
            let result = waf.analyze_request(req.request());

            // Check event rate limit
            if !waf.check_event_rate_limit(&result.client_ip) {
                warn!("Event rate limit exceeded for IP: {}", result.client_ip);
            }

            // Log event
            waf.log_event(&result, req.request());

            // Handle action based on mode
            match (waf.config.mode.as_str(), &result.action) {
                ("shadow", WafAction::Block) => {
                    // In shadow mode, log but don't block
                    warn!(
                        "WAF would block request in block mode: IP={}, Score={}",
                        result.client_ip, result.score
                    );
                    if result.score >= waf.config.grey_threshold {
                        waf.add_to_grey(&result.client_ip);
                    }
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                ("shadow", WafAction::Grey) => {
                    waf.add_to_grey(&result.client_ip);
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                ("shadow", WafAction::Pass) => {
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                ("block", WafAction::Block) => {
                    // Block the request
                    waf.add_to_ban(&result.client_ip);
                    let error_response = HttpResponse::build(StatusCode::FORBIDDEN)
                        .json(json!({
                            "error": "forbidden",
                            "reason": "waf_block",
                            "score": result.score
                        }));
                    Ok(ServiceResponse::new(req.into_parts().0, error_response).map_into_right_body())
                }
                ("block", WafAction::Grey) => {
                    waf.add_to_grey(&result.client_ip);
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                ("block", WafAction::Pass) => {
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                _ => {
                    // Unknown mode, pass through
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
            }
        })
    }
}
