use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error as ActixError, HttpMessage,
};
use actix_web::web;
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::rc::Rc;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::models::{AppState, AppError};
use deadpool_redis::redis::AsyncCommands;
use sqlx::PgPool;
pub struct WalletAuthMiddleware;
impl<S, B> Transform<S, ServiceRequest> for WalletAuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type InitError = ();
    type Transform = WalletAuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(WalletAuthMiddlewareService { service: Rc::new(service) }))
    }
}
pub struct WalletAuthMiddlewareService<S> {
    service: Rc<S>,
}
impl<S, B> Service<ServiceRequest> for WalletAuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
    forward_ready!(service);
    fn call(&self, req: ServiceRequest) -> Self::Future {
        let state = req.app_data::<web::Data<AppState>>().expect("AppState").clone();
        let service = self.service.clone();
        let headers = req.headers();
        let pubkey = match headers.get("x-wallet-pubkey").and_then(|v| v.to_str().ok()) {
            Some(v) => v.to_string(),
            None => return Box::pin(async { Err(AppError::Unauthorized("Missing x-wallet-pubkey".into()).into()) }),
        };
        let _signature = match headers.get("x-wallet-signature").and_then(|v| v.to_str().ok()) {
            Some(v) => v,
            None => return Box::pin(async { Err(AppError::Unauthorized("Missing x-wallet-signature".into()).into()) }),
        };
        let timestamp = match headers.get("x-timestamp").and_then(|v| v.to_str().ok().and_then(|v| v.parse::<i64>().ok())) {
            Some(v) => v,
            None => return Box::pin(async { Err(AppError::Unauthorized("Missing x-timestamp".into()).into()) }),
        };
        let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs() as i64,
            Err(_) => return Box::pin(async { Err(AppError::InternalServerError("Time error".into()).into()) }),
        };
        if (timestamp - now).abs() > 300 {
            return Box::pin(async { Err(AppError::Unauthorized("Timestamp expired".into()).into()) });
        }
        let pubkey_obj = match Pubkey::from_str(&pubkey) {
            Ok(pubkey) => pubkey,
            Err(_) => return Box::pin(async { Err(AppError::Unauthorized("Invalid pubkey".into()).into()) }),
        };
        Box::pin(async move {
            let mut conn = state.redis.get().await.map_err(|e| AppError::CacheError(e.to_string()))?;
            let nonce_key = format!("nonce:{}", pubkey);
            let nonce: Option<String> = conn.get(&nonce_key).await.map_err(|e| AppError::CacheError(e.to_string()))?;
            if nonce.is_none() {
                return Err(AppError::Unauthorized("Invalid nonce".into()).into());
            }
            let rl_key = format!("rl:{}", pubkey);
            let count: i64 = conn.incr(&rl_key, 1).await.map_err(|e| AppError::CacheError(e.to_string()))?;
            conn.expire::<_, ()>(&rl_key, state.config.rate_limit.user_window_secs as usize).await.map_err(|e| AppError::CacheError(e.to_string()))?;
            if count > state.config.rate_limit.user_max_requests as i64 {
                return Err(AppError::TooManyRequests("Rate limit".into()).into());
            }
            conn.del::<_, ()>(&nonce_key).await.map_err(|e| AppError::CacheError(e.to_string()))?;
            req.extensions_mut().insert(pubkey_obj);
            service.call(req).await
        })
    }
}
pub async fn require_permission(pubkey: &Pubkey, endpoint: &str, pool: &PgPool) -> Result<(), AppError> {
    let exists = sqlx::query_scalar!(
        "SELECT 1 FROM user_permissions WHERE pubkey = $1 AND endpoint = $2 AND permission = 'allow'",
        pubkey.to_string(),
        endpoint
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    if exists.is_none() {
        return Err(AppError::Unauthorized("Permission denied".into()));
    }
    Ok(())
}
