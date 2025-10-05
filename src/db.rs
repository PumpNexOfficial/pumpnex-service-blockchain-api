use crate::models::AppError;
use solana_sdk::pubkey::Pubkey;
use sqlx::PgPool;

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
