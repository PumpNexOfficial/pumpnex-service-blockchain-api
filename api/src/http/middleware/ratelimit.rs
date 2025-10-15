/// Rate limiting middleware
/// 
/// Fixed window in-memory strategy with IP and User (X-Wallet-Address) tracking

use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use serde::Serialize;
use std::{
    collections::HashMap,
    future::{ready, Ready},
    net::IpAddr,
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::config::RateLimitConfig;

#[derive(Serialize)]
struct RateLimitErrorResponse {
    error: String,
    retry_after: u64,
}

#[derive(Clone)]
struct WindowEntry {
    count: u32,
    window_start: Instant,
}

type RateLimitStore = Arc<Mutex<HashMap<String, WindowEntry>>>;

#[derive(Clone)]
pub struct RateLimit {
    config: RateLimitConfig,
    store: RateLimitStore,
}

impl RateLimit {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn extract_client_ip(req: &ServiceRequest, respect_xff: bool) -> Option<IpAddr> {
        if respect_xff {
            if let Some(xff) = req.headers().get("x-forwarded-for") {
                if let Ok(xff_str) = xff.to_str() {
                    // Parse first valid IP from X-Forwarded-For
                    for ip_str in xff_str.split(',') {
                        if let Ok(ip) = ip_str.trim().parse::<IpAddr>() {
                            return Some(ip);
                        }
                    }
                }
            }
        }

        // Fallback to peer_addr
        req.peer_addr().map(|addr| addr.ip())
    }

    fn extract_user_id(req: &ServiceRequest) -> Option<String> {
        req.headers()
            .get("x-wallet-address")
            .and_then(|h| h.to_str().ok())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    fn is_whitelisted_path(path: &str) -> bool {
        matches!(path, "/healthz" | "/readyz")
    }

    fn check_limit(
        &self,
        key: String,
        max_requests: u32,
        window_duration: Duration,
    ) -> Result<(), u64> {
        let mut store = self.store.lock().unwrap();
        let now = Instant::now();

        let entry = store.entry(key).or_insert_with(|| WindowEntry {
            count: 0,
            window_start: now,
        });

        // Check if window expired
        if now.duration_since(entry.window_start) >= window_duration {
            entry.count = 0;
            entry.window_start = now;
        }

        // Check limit
        if entry.count >= max_requests {
            let elapsed = now.duration_since(entry.window_start);
            let retry_after = window_duration.saturating_sub(elapsed).as_secs();
            return Err(retry_after);
        }

        entry.count += 1;
        Ok(())
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimit
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddleware {
            service: Rc::new(service),
            config: self.config.clone(),
            store: self.store.clone(),
        }))
    }
}

pub struct RateLimitMiddleware<S> {
    service: Rc<S>,
    config: RateLimitConfig,
    store: RateLimitStore,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Skip if disabled
        if !self.config.enabled {
            let service = self.service.clone();
            return Box::pin(async move {
                let res = service.call(req).await?;
                Ok(res.map_into_left_body())
            });
        }

        // Skip whitelisted paths
        let path = req.path().to_string();
        if RateLimit::is_whitelisted_path(&path) {
            let service = self.service.clone();
            return Box::pin(async move {
                let res = service.call(req).await?;
                Ok(res.map_into_left_body())
            });
        }

        let method = req.method().to_string();
        let config = self.config.clone();
        let store = self.store.clone();

        // Extract identifiers
        let client_ip = RateLimit::extract_client_ip(&req, config.respect_x_forwarded_for);
        let user_id = RateLimit::extract_user_id(&req);

        // IP-based rate limiting
        if let Some(ip) = client_ip {
            let ip_key = format!("ip:{}", ip);
            let window_duration = Duration::from_secs(config.ip_window_secs);

            let result = {
                let mut store_lock = store.lock().unwrap();
                let now = Instant::now();

                let entry = store_lock.entry(ip_key.clone()).or_insert_with(|| WindowEntry {
                    count: 0,
                    window_start: now,
                });

                if now.duration_since(entry.window_start) >= window_duration {
                    entry.count = 0;
                    entry.window_start = now;
                }

                if entry.count >= config.ip_max_requests {
                    let elapsed = now.duration_since(entry.window_start);
                    let retry_after = window_duration.saturating_sub(elapsed).as_secs();
                    Err(retry_after)
                } else {
                    entry.count += 1;
                    Ok(())
                }
            };

            if let Err(retry_after) = result {
                tracing::warn!(
                    scope = "ip",
                    key = %ip,
                    limit = config.ip_max_requests,
                    window = config.ip_window_secs,
                    retry_after = retry_after,
                    method = %method,
                    path = %path,
                    "Rate limit exceeded"
                );

                let response = HttpResponse::TooManyRequests()
                    .insert_header(("Retry-After", retry_after.to_string()))
                    .json(RateLimitErrorResponse {
                        error: "rate_limited".to_string(),
                        retry_after,
                    });

                let (req, _) = req.into_parts();
                return Box::pin(async move {
                    Ok(ServiceResponse::new(req, response).map_into_right_body())
                });
            }
        } else {
            tracing::warn!("Unable to determine client IP for rate limiting");
        }

        // User-based rate limiting (if wallet address present)
        if let Some(user) = user_id {
            let user_key = format!("user:{}", user);
            let window_duration = Duration::from_secs(config.user_window_secs);

            let result = {
                let mut store_lock = store.lock().unwrap();
                let now = Instant::now();

                let entry = store_lock.entry(user_key.clone()).or_insert_with(|| WindowEntry {
                    count: 0,
                    window_start: now,
                });

                if now.duration_since(entry.window_start) >= window_duration {
                    entry.count = 0;
                    entry.window_start = now;
                }

                if entry.count >= config.user_max_requests {
                    let elapsed = now.duration_since(entry.window_start);
                    let retry_after = window_duration.saturating_sub(elapsed).as_secs();
                    Err(retry_after)
                } else {
                    entry.count += 1;
                    Ok(())
                }
            };

            if let Err(retry_after) = result {
                tracing::warn!(
                    scope = "user",
                    key = %user,
                    limit = config.user_max_requests,
                    window = config.user_window_secs,
                    retry_after = retry_after,
                    method = %method,
                    path = %path,
                    "Rate limit exceeded"
                );

                let response = HttpResponse::TooManyRequests()
                    .insert_header(("Retry-After", retry_after.to_string()))
                    .json(RateLimitErrorResponse {
                        error: "rate_limited".to_string(),
                        retry_after,
                    });

                let (req, _) = req.into_parts();
                return Box::pin(async move {
                    Ok(ServiceResponse::new(req, response).map_into_right_body())
                });
            }
        }

        // Pass through if all limits OK
        let service = self.service.clone();
        Box::pin(async move {
            let res = service.call(req).await?;
            Ok(res.map_into_left_body())
        })
    }
}

