use actix_web::{web, HttpRequest, HttpResponse, Result as ActixResult, HttpMessage};
use crate::models::{AppState, SolanaTransactionFilter, PaginationQuery, AppError, SolanaTransaction, NonceRequest};
use crate::utils::process_solana_transactions;
use crate::middleware::require_permission;
use sha2::{Sha256, Digest};
use deadpool_redis::redis::AsyncCommands;
use sqlx::Arguments;
use sqlx::postgres::PgArguments;

// 🔥 Делаем функцию публичной для main.rs и тестов
pub use crate::routes::auth::get_nonce;

pub async fn get_transactions(
    state: web::Data<AppState>,
    query: web::Query<SolanaTransactionFilter>,
    pagination: web::Query<PaginationQuery>,
    req: HttpRequest,
) -> ActixResult<HttpResponse, AppError> {
    let pubkey = req
        .extensions()
        .get::<solana_sdk::pubkey::Pubkey>()
        .ok_or(AppError::Unauthorized("No pubkey".to_string()))?
        .clone();

    require_permission(&pubkey, "/api/transactions", &state.db).await?;

    let cache_key = {
        let input = serde_json::to_string(&(&query.0, &pagination.0)).unwrap();
        format!("tx:{:x}", Sha256::digest(input.as_bytes()))
    };

    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| AppError::CacheError(e.to_string()))?;

    if let Some(cached) = conn
        .get::<_, Option<String>>(&cache_key)
        .await
        .map_err(|e| AppError::CacheError(e.to_string()))?
    {
        state.metrics.cache_hits.with_label_values(&["redis"]).inc();
        let data: Vec::<SolanaTransaction> =
            serde_json::from_str(&cached).map_err(|e| AppError::SerializationError(e.to_string()))?;
        return Ok(HttpResponse::Ok().json(data));
    }

    let mut sql = "SELECT id, signature, from_pubkey, to_pubkey, instructions, lamports, timestamp, slot FROM solana_transactions".to_string();
    let mut conditions = Vec::new();
    let mut args = PgArguments::default();

    if let Some(ref v) = query.from_pubkey {
        conditions.push(format!("from_pubkey = $1"));
        args.add(v.clone());
    }
    if let Some(ref v) = query.to_pubkey {
        conditions.push(format!("to_pubkey = $2"));
        args.add(v.clone());
    }
    if let Some(v) = query.min_lamports {
        conditions.push(format!("lamports >= $3"));
        args.add(v);
    }
    if let Some(v) = query.start_slot {
        conditions.push(format!("slot >= $4"));
        args.add(v);
    }
    if let Some(v) = query.end_slot {
        conditions.push(format!("slot <= $5"));
        args.add(v);
    }
    if let Some(ref v) = query.program_id {
        conditions.push(format!("instructions @> $6::jsonb"));
        args.add(serde_json::json!([{"program_id": v}]));
    }
    if let Some(ref v) = query.instruction_type {
        conditions.push(format!("instructions @> $7::jsonb"));
        args.add(serde_json::json!([{"instruction_type": v}]));
    }

    if !conditions.is_empty() {
        sql.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
    }

    let limit = pagination.limit.unwrap_or(10).min(100);
    let offset = pagination.offset.unwrap_or(0);
    sql.push_str(&format!(
        " ORDER BY timestamp DESC LIMIT {} OFFSET {}",
        limit, offset
    ));

    let rows = sqlx::query_with(&sql, args)
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let transactions = process_solana_transactions(rows)?;
    let json_str =
        serde_json::to_string(&transactions).map_err(|e| AppError::SerializationError(e.to_string()))?;

    let _ = conn
        .set_ex::<_, _, ()>(&cache_key, &json_str, state.config.cache.solana_data_ttl_secs as usize)
        .await;

    Ok(HttpResponse::Ok().json(transactions))
}

pub async fn post_nonce(
    state: web::Data<AppState>,
    req: web::Json<NonceRequest>,
) -> ActixResult<HttpResponse, AppError> {
    get_nonce(req, state).await
}
