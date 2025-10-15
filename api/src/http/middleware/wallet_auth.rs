/// Wallet authentication middleware
/// 
/// Verifies Ed25519 signatures from Solana wallets

use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use serde::Serialize;
use std::{
    future::{ready, Ready},
    rc::Rc,
};

use crate::app_state::AppState;
use crate::config::AuthConfig;

#[derive(Serialize)]
struct AuthErrorResponse {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    missing: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct WalletAuth {
    config: AuthConfig,
}

impl WalletAuth {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    fn is_bypassed(&self, path: &str) -> bool {
        self.config.bypass_paths.iter().any(|bp| path == bp)
    }

    fn is_protected(&self, path: &str) -> bool {
        self.config.protect_prefixes.iter().any(|prefix| path.starts_with(prefix))
    }
}

impl<S, B> Transform<S, ServiceRequest> for WalletAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = WalletAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(WalletAuthMiddleware {
            service: Rc::new(service),
            config: self.config.clone(),
        }))
    }
}

pub struct WalletAuthMiddleware<S> {
    service: Rc<S>,
    config: AuthConfig,
}

impl<S> WalletAuthMiddleware<S> {
    fn is_bypassed(&self, path: &str) -> bool {
        self.config.bypass_paths.iter().any(|bp| path == bp)
    }

    fn is_protected(&self, path: &str) -> bool {
        self.config.protect_prefixes.iter().any(|prefix| path.starts_with(prefix))
    }
}

impl<S, B> Service<ServiceRequest> for WalletAuthMiddleware<S>
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

        let path = req.path().to_string();
        let path_with_query = req.uri().path_and_query()
            .map(|pq| pq.as_str().to_string())
            .unwrap_or_else(|| path.clone());

        // Skip bypassed paths
        if self.is_bypassed(&path) {
            let service = self.service.clone();
            return Box::pin(async move {
                let res = service.call(req).await?;
                Ok(res.map_into_left_body())
            });
        }

        // Skip unprotected paths
        if !self.is_protected(&path) {
            let service = self.service.clone();
            return Box::pin(async move {
                let res = service.call(req).await?;
                Ok(res.map_into_left_body())
            });
        }

        // Extract headers
        let config = self.config.clone();
        let method = req.method().to_string();

        let wallet_address = req.headers()
            .get(&config.header_wallet_address)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let wallet_signature = req.headers()
            .get(&config.header_wallet_signature)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let wallet_nonce = req.headers()
            .get(&config.header_wallet_nonce)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        // Check required headers
        if wallet_address.is_none() || wallet_signature.is_none() || wallet_nonce.is_none() {
            let mut missing = Vec::new();
            if wallet_address.is_none() {
                missing.push(config.header_wallet_address.clone());
            }
            if wallet_signature.is_none() {
                missing.push(config.header_wallet_signature.clone());
            }
            if wallet_nonce.is_none() {
                missing.push(config.header_wallet_nonce.clone());
            }

            let response = HttpResponse::BadRequest().json(AuthErrorResponse {
                error: "bad_request".to_string(),
                reason: None,
                missing: Some(missing),
            });

            let (req, _) = req.into_parts();
            return Box::pin(async move {
                Ok(ServiceResponse::new(req, response).map_into_right_body())
            });
        }

        let address = wallet_address.unwrap();
        let signature = wallet_signature.unwrap();
        let nonce = wallet_nonce.unwrap();

        // Get Redis connection from state
        let redis_conn = req.app_data::<actix_web::web::Data<AppState>>()
            .and_then(|state| state.redis.clone());

        let service = self.service.clone();

        Box::pin(async move {
            // Check Redis connection
            let mut redis_conn = match redis_conn {
                Some(conn) => conn,
                None => {
                    tracing::error!("Redis not available for auth");
                    let response = HttpResponse::InternalServerError().json(AuthErrorResponse {
                        error: "internal".to_string(),
                        reason: Some("redis_unavailable".to_string()),
                        missing: None,
                    });
                    let (req, _) = req.into_parts();
                    return Ok(ServiceResponse::new(req, response).map_into_right_body());
                }
            };

            // Verify nonce in Redis
            let redis_key = format!("{}:{}", config.redis_key_prefix, address);
            let stored_nonce: Option<String> = match redis::cmd("GET")
                .arg(&redis_key)
                .query_async(&mut redis_conn)
                .await
            {
                Ok(n) => n,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get nonce from Redis");
                    let response = HttpResponse::InternalServerError().json(AuthErrorResponse {
                        error: "internal".to_string(),
                        reason: Some("redis_error".to_string()),
                        missing: None,
                    });
                    let (req, _) = req.into_parts();
                    return Ok(ServiceResponse::new(req, response).map_into_right_body());
                }
            };

            // Check nonce exists
            let stored_nonce = match stored_nonce {
                Some(n) => n,
                None => {
                    tracing::warn!(
                        address = %address,
                        path = %path,
                        "Nonce not found or expired"
                    );
                    let response = HttpResponse::Unauthorized().json(AuthErrorResponse {
                        error: "unauthorized".to_string(),
                        reason: Some("nonce_missing".to_string()),
                        missing: None,
                    });
                    let (req, _) = req.into_parts();
                    return Ok(ServiceResponse::new(req, response).map_into_right_body());
                }
            };

            // Check nonce matches
            if stored_nonce != nonce {
                tracing::warn!(
                    address = %address,
                    path = %path,
                    "Nonce mismatch"
                );
                let response = HttpResponse::Unauthorized().json(AuthErrorResponse {
                    error: "unauthorized".to_string(),
                    reason: Some("nonce_mismatch".to_string()),
                    missing: None,
                });
                let (req, _) = req.into_parts();
                return Ok(ServiceResponse::new(req, response).map_into_right_body());
            }

            // Decode public key
            let pubkey = match blockchain_auth::decode_pubkey_b58(&address) {
                Ok(pk) => pk,
                Err(e) => {
                    tracing::warn!(error = %e, address = %address, "Invalid public key");
                    let response = HttpResponse::BadRequest().json(AuthErrorResponse {
                        error: "bad_request".to_string(),
                        reason: Some(format!("invalid_pubkey: {}", e)),
                        missing: None,
                    });
                    let (req, _) = req.into_parts();
                    return Ok(ServiceResponse::new(req, response).map_into_right_body());
                }
            };

            // Decode signature
            let sig_bytes = if config.accept_signature_b58 {
                blockchain_auth::decode_sig_b58(&signature)
            } else if config.accept_signature_b64 {
                blockchain_auth::decode_sig_b64(&signature)
            } else {
                Err(blockchain_auth::AuthError::InvalidBase58("No signature format enabled".to_string()))
            };

            let sig_bytes = match sig_bytes {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(error = %e, "Invalid signature format");
                    let response = HttpResponse::BadRequest().json(AuthErrorResponse {
                        error: "bad_request".to_string(),
                        reason: Some(format!("invalid_signature: {}", e)),
                        missing: None,
                    });
                    let (req, _) = req.into_parts();
                    return Ok(ServiceResponse::new(req, response).map_into_right_body());
                }
            };

            // Build signing string
            let signing_string = blockchain_auth::build_signing_string(
                &method,
                &path_with_query,
                &nonce,
                &config.canonicalize_method,
                &config.canonicalize_path,
            );

            // Verify signature
            let is_valid = match blockchain_auth::verify_ed25519(&pubkey, signing_string.as_bytes(), &sig_bytes) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(error = %e, "Signature verification error");
                    let response = HttpResponse::InternalServerError().json(AuthErrorResponse {
                        error: "internal".to_string(),
                        reason: Some("verification_error".to_string()),
                        missing: None,
                    });
                    let (req, _) = req.into_parts();
                    return Ok(ServiceResponse::new(req, response).map_into_right_body());
                }
            };

            if !is_valid {
                tracing::warn!(
                    address = %address,
                    method = %method,
                    path = %path,
                    "Invalid signature"
                );
                let response = HttpResponse::Unauthorized().json(AuthErrorResponse {
                    error: "unauthorized".to_string(),
                    reason: Some("invalid_signature".to_string()),
                    missing: None,
                });
                let (req, _) = req.into_parts();
                return Ok(ServiceResponse::new(req, response).map_into_right_body());
            }

            // Delete nonce (one-time use)
            let _: () = redis::cmd("DEL")
                .arg(&redis_key)
                .query_async(&mut redis_conn)
                .await
                .unwrap_or_default();

            tracing::info!(
                address = %address,
                method = %method,
                path = %path,
                "Authentication successful"
            );

            // Pass through
            let res = service.call(req).await?;
            Ok(res.map_into_left_body())
        })
    }
}

