use actix_web::{web, HttpRequest, HttpResponse, Result as ActixResult};
use crate::models::{AppState, NonceRequest, NonceResponse, AppError};
use rand::{distributions::Alphanumeric, Rng};
use deadpool_redis::redis::AsyncCommands;
use ed25519_dalek::{PublicKey, Verifier, Signature};
pub async fn get_nonce(
    req: web::Json<NonceRequest>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse, AppError> {
    let wallet = &req.wallet_address;
    if wallet.is_empty() {
        return Err(AppError::BadRequest("Invalid wallet address".to_string()).into());
    }
    let nonce: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let key = format!("nonce:{}", wallet);
    let mut conn = state.redis.get().await.map_err(|e| AppError::CacheError(e.to_string()))?;
    conn.set_ex::<_, _, ()>(&key, &nonce, state.config.cache.nonce_ttl_secs as usize)
        .await
        .map_err(|e| AppError::CacheError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(NonceResponse { nonce }))
}
pub async fn verify_nonce(
    req: HttpRequest,
    body: web::Json<NonceRequest>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse, AppError> {
    let wallet = &body.wallet_address;
    if wallet.is_empty() {
        return Err(AppError::BadRequest("Invalid wallet address".to_string()).into());
    }
    let signature_str = req.headers().get("x-wallet-signature").and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing x-wallet-signature".to_string()))?;
    let nonce_key = format!("nonce:{}", wallet);
    let mut conn = state.redis.get().await.map_err(|e| AppError::CacheError(e.to_string()))?;
    let nonce: Option<String> = conn.get(&nonce_key).await.map_err(|e| AppError::CacheError(e.to_string()))?;
    let nonce = nonce.ok_or_else(|| AppError::Unauthorized("Invalid nonce".to_string()))?;
    let pubkey_bytes = bs58::decode(wallet)
        .into_vec()
        .map_err(|_| AppError::BadRequest("Invalid pubkey base58".to_string()))?;
    let pubkey = PublicKey::from_bytes(&pubkey_bytes)
        .map_err(|_| AppError::BadRequest("Invalid public key bytes".to_string()))?;
    let signature_bytes = bs58::decode(signature_str)
        .into_vec()
        .map_err(|_| AppError::BadRequest("Invalid signature base58".to_string()))?;
    let signature = Signature::from_bytes(&signature_bytes)
        .map_err(|_| AppError::BadRequest("Invalid signature bytes".to_string()))?;
    pubkey.verify(nonce.as_bytes(), &signature)
        .map_err(|_| AppError::Unauthorized("Signature verification failed".to_string()))?;
    conn.del::<_, ()>(&nonce_key).await.map_err(|e| AppError::CacheError(e.to_string()))?;
    Ok(HttpResponse::Ok().json("Signature verified"))
}
