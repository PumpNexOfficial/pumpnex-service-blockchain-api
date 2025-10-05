use actix_web::{get, web, HttpResponse, Responder};
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

    let mut first = true;
    if let Some(pubkey) = query_params.from_pubkey {
        query_builder.push(if first { " WHERE " } else { " AND " });
        query_builder.push("from_pubkey = ").push_bind(pubkey);
        first = false;
    }
    if let Some(pubkey) = query_params.to_pubkey {
        query_builder.push(if first { " WHERE " } else { " AND " });
        query_builder.push("to_pubkey = ").push_bind(pubkey);
        first = false;
    }
    if let Some(min) = query_params.min_lamports {
        query_builder.push(if first { " WHERE " } else { " AND " });
        query_builder.push("lamports >= ").push_bind(min);
        first = false;
    }
    if let Some(start) = query_params.start_slot {
        query_builder.push(if first { " WHERE " } else { " AND " });
        query_builder.push("slot >= ").push_bind(start);
        first = false;
    }
    if let Some(end) = query_params.end_slot {
        query_builder.push(if first { " WHERE " } else { " AND " });
        query_builder.push("slot <= ").push_bind(end);
        first = false;
    }
    if let Some(program) = query_params.program_id {
        query_builder.push(if first { " WHERE " } else { " AND " });
        query_builder.push("instructions @> ").push_bind(json!([{"program_id": program}]));
    }

    let limit = pagination_params.limit.unwrap_or(config.pagination.default_limit)
        .min(config.pagination.max_limit);
    query_builder.push(" LIMIT ").push_bind(limit);
    query_builder.push(" OFFSET ").push_bind(pagination_params.offset.unwrap_or(0));
    Ok(query_builder)
}

#[get("/transactions")]
pub async fn get_transactions(
    state: web::Data<AppState>,
    query_params: web::Query<SolanaTransactionFilter>,
    pagination_params: web::Query<PaginationQuery>,
) -> impl Responder {
    if query_params.0.validate().is_err() || pagination_params.0.validate().is_err() {
        return Err(AppError::BadRequest("Invalid parameters".to_string()));
    }
    let mut query_builder = match build_transaction_query(
        query_params.0.clone(),
        pagination_params.0.clone(),
        &state.config
    ) {
        Ok(builder) => builder,
        Err(e) => return Err(e),
    };
    let transactions = query_builder
        .build_query_as::<SolanaTransaction>()
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::InternalServerError(format!("DB error: {}", e)))?;
    info!("Fetched {} transactions", transactions.len());
    Ok(HttpResponse::Ok().json(transactions))
}
