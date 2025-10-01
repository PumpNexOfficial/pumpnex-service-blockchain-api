use actix_web::{get, web, HttpResponse, Error as ActixError};
use crate::models::{SolanaTransaction, SolanaTransactionFilter, PaginationQuery, AppConfig, AppError};
use sqlx::QueryBuilder;
use tracing::{info};
use validator::Validate;

fn build_transaction_query(
    query_params: SolanaTransactionFilter,
    pagination_params: PaginationQuery,
    config: &AppConfig,
) -> Result<QueryBuilder<sqlx::Postgres>, AppError> {
    let mut query_builder: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        "SELECT id, signature, from_pubkey, to_pubkey, instructions, lamports, timestamp FROM solana_transactions"
    );
    let mut has_condition = false;
    if let Some(pubkey) = query_params.from_pubkey.clone() {
        query_builder.push(" WHERE from_pubkey = ").push_bind(pubkey);
        has_condition = true;
    }
    if let Some(pubkey) = query_params.to_pubkey.clone() {
        if has_condition {
            query_builder.push(" AND ");
        } else {
            query_builder.push(" WHERE ");
            has_condition = true;
        }
        query_builder.push("to_pubkey = ").push_bind(pubkey);
    }
    if let Some(min) = query_params.min_lamports {
        if has_condition {
            query_builder.push(" AND ");
        } else {
            query_builder.push(" WHERE ");
            has_condition = true;
        }
        query_builder.push("lamports >= ").push_bind(min);
    }
    if let Some(start) = query_params.start_slot {
        if has_condition {
            query_builder.push(" AND ");
        } else {
            query_builder.push(" WHERE ");
            has_condition = true;
        }
        query_builder.push("slot >= ").push_bind(start);
    }
    if let Some(end) = query_params.end_slot {
        if has_condition {
            query_builder.push(" AND ");
        } else {
            query_builder.push(" WHERE ");
        }
        query_builder.push("slot <= ").push_bind(end);
    }
    let limit = pagination_params.limit.unwrap_or(config.pagination.default_limit)
        .min(config.pagination.max_limit);
    query_builder.push(" LIMIT ").push_bind(limit);
    query_builder.push(" OFFSET ").push_bind(pagination_params.offset.unwrap_or(0));
    Ok(query_builder)
}

#[get("/transactions")]
pub async fn get_transactions(
    state: web::Data<crate::models::AppState>,
    query_params: web::Query<SolanaTransactionFilter>,
    pagination_params: web::Query<PaginationQuery>,
) -> Result<HttpResponse, ActixError> {
    if query_params.0.validate().is_err() || pagination_params.0.validate().is_err() {
        return Err(AppError::BadRequest("Invalid parameters".to_string()).into());
    }
    let mut query_builder = build_transaction_query(query_params.0, pagination_params.0, &state.config)?;
    let transactions = query_builder
        .build_query_as::<SolanaTransaction>()
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::InternalServerError(format!("DB error: {}", e)))?;
    info!("Fetched {} transactions", transactions.len());
    Ok(HttpResponse::Ok().json(transactions))
}
