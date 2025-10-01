use actix_web::{get, web, HttpResponse, Error as ActixError};
use crate::models::{SolanaTransaction, SolanaTransactionFilter, PaginationQuery, AppConfig, AppError, AppState};
use sqlx::QueryBuilder;
use tracing::info;
use validator::Validate;
use serde_json::json;

fn build_transaction_query(
    query_params: SolanaTransactionFilter,
    pagination_params: PaginationQuery,
    config: &AppConfig,
) -> Result<QueryBuilder<sqlx::Postgres>, AppError> {
    let mut query_builder: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        "SELECT id, signature, from_pubkey, to_pubkey, instructions, lamports, slot, timestamp FROM solana_transactions"
    );
    let mut has_condition = false;

    if let Some(pubkey) = query_params.from_pubkey {
        query_builder.push(" WHERE from_pubkey = ").push_bind(pubkey);
        has_condition = true;
    }
    if let Some(pubkey) = query_params.to_pubkey {
        if has_condition {
            query_builder.push(" AND to_pubkey = ").push_bind(pubkey);
        } else {
            query_builder.push(" WHERE to_pubkey = ").push_bind(pubkey);
            has_condition = true;
        }
    }
    if let Some(min) = query_params.min_lamports {
        if has_condition {
            query_builder.push(" AND lamports >= ").push_bind(min);
        } else {
            query_builder.push(" WHERE lamports >= ").push_bind(min);
            has_condition = true;
        }
    }
    if let Some(start) = query_params.start_slot {
        if has_condition {
            query_builder.push(" AND slot >= ").push_bind(start);
        } else {
            query_builder.push(" WHERE slot >= ").push_bind(start);
            has_condition = true;
        }
    }
    if let Some(end) = query_params.end_slot {
        if has_condition {
            query_builder.push(" AND slot <= ").push_bind(end);
        } else {
            query_builder.push(" WHERE slot <= ").push_bind(end);
            has_condition = true;
        }
    }
    if let Some(program) = query_params.program_id {
        if has_condition {
            query_builder.push(" AND instructions @> ").push_bind(json!([{ "program_id": program }]));
        } else {
            query_builder.push(" WHERE instructions @> ").push_bind(json!([{ "program_id": program }]));
            has_condition = true;
        }
    }
    let limit = pagination_params.limit.unwrap_or(config.pagination.default_limit)
        .min(config.pagination.max_limit);
    query_builder.push(" LIMIT ").push_bind(limit);
    query_builder.push(" OFFSET ").push_bind(pagination_params.offset.unwrap_or(0));
    Ok(query_builder)
}

#[get("/api/transactions")]
pub async fn get_transactions(
    state: web::Data<AppState>,
    query_params: web::Query<SolanaTransactionFilter>,
    pagination_params: web::Query<PaginationQuery>,
) -> Result<HttpResponse, ActixError> {
    if query_params.0.validate().is_err() || pagination_params.0.validate().is_err() {
        return Err(AppError::BadRequest("Invalid parameters".to_string()).into());
    }
    let mut query_builder = build_transaction_query(
        query_params.0.clone(),
        pagination_params.0.clone(),
        &state.config
    )?;
    let transactions = query_builder
        .build_query_as::<SolanaTransaction>()
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::InternalServerError(format!("DB error: {}", e)))?;
    info!("Fetched {} transactions", transactions.len());
    Ok(HttpResponse::Ok().json(transactions))
}
