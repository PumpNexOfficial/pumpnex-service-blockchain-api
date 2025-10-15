use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::sync::Arc;

use crate::app_state::AppState;
use crate::cache::{Cache, CachedResponse};
use crate::config::CacheConfig;
use crate::errors::ApiError;
use crate::repository::transactions::{
    NewTransaction, Pagination, SolanaTransaction, TransactionFilter, TransactionRepository,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub signature: Option<String>,
    #[serde(rename = "from")]
    pub from_pubkey: Option<String>,
    #[serde(rename = "to")]
    pub to_pubkey: Option<String>,
    pub program_id: Option<String>,
    pub slot_from: Option<i64>,
    pub slot_to: Option<i64>,
    #[serde(default = "default_sort_by")]
    pub sort_by: String, // "slot" | "signature" | "block_time"
    #[serde(default = "default_order")]
    pub order: String, // "asc" | "desc"
    #[serde(default = "default_limit")]
    pub limit: u32, // 1..=200
    #[serde(default)]
    pub offset: u32,
}

fn default_sort_by() -> String {
    "slot".to_string()
}

fn default_order() -> String {
    "desc".to_string()
}

fn default_limit() -> u32 {
    50
}

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub items: Vec<SolanaTransaction>,
    pub page: PageInfo,
    pub sort: SortInfo,
}

#[derive(Debug, Serialize)]
pub struct PageInfo {
    pub limit: u32,
    pub offset: u32,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct SortInfo {
    pub by: String,
    pub order: String,
}

// Validate query parameters
fn validate_query(query: &ListQuery) -> Result<(), ApiError> {
    // Validate limit
    if query.limit < 1 || query.limit > 200 {
        return Err(ApiError::BadRequest {
            missing: vec![],
            reason: Some("limit must be between 1 and 200".to_string()),
        });
    }

    // Validate sort_by
    if !["slot", "signature", "block_time"].contains(&query.sort_by.as_str()) {
        return Err(ApiError::BadRequest {
            missing: vec![],
            reason: Some("sort_by must be one of: slot, signature, block_time".to_string()),
        });
    }

    // Validate order
    if !["asc", "desc"].contains(&query.order.as_str()) {
        return Err(ApiError::BadRequest {
            missing: vec![],
            reason: Some("order must be one of: asc, desc".to_string()),
        });
    }

    // Validate slot range
    if let (Some(from), Some(to)) = (query.slot_from, query.slot_to) {
        if from > to {
            return Err(ApiError::BadRequest {
                missing: vec![],
                reason: Some("slot_from must be <= slot_to".to_string()),
            });
        }
    }

    Ok(())
}

// Compute ETag based on query params and summary stats
fn compute_etag(
    query: &ListQuery,
    total: i64,
    max_slot: i64,
    max_created_at: chrono::DateTime<chrono::Utc>,
    salt: &str,
) -> String {
    let mut hasher = Sha1::new();

    // Serialize query params
    let query_str = format!(
        "sig={:?}|from={:?}|to={:?}|prog={:?}|slot_from={:?}|slot_to={:?}|sort={}|order={}|limit={}|offset={}",
        query.signature,
        query.from_pubkey,
        query.to_pubkey,
        query.program_id,
        query.slot_from,
        query.slot_to,
        query.sort_by,
        query.order,
        query.limit,
        query.offset
    );

    hasher.update(query_str.as_bytes());
    hasher.update(b"|");
    hasher.update(total.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(max_slot.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(max_created_at.to_rfc3339().as_bytes());
    hasher.update(b"|");
    hasher.update(salt.as_bytes());

    let result = hasher.finalize();
    format!("W/\"{:x}\"", result)
}

// GET /api/transactions
pub async fn list_transactions(
    req: HttpRequest,
    query: web::Query<ListQuery>,
    app_state: web::Data<AppState>,
    cache_config: web::Data<CacheConfig>,
    cache: web::Data<Arc<dyn Cache>>,
) -> Result<impl Responder, ApiError> {
    // Validate query
    validate_query(&query)?;

    let pg_pool = app_state
        .postgres
        .as_ref()
        .ok_or_else(|| ApiError::ServiceUnavailable {
            details: "Database not available".to_string(),
        })?;

    let repo = TransactionRepository::new(pg_pool.clone());

    // Build filter
    let filter = TransactionFilter {
        signature: query.signature.clone(),
        from_pubkey: query.from_pubkey.clone(),
        to_pubkey: query.to_pubkey.clone(),
        program_id: query.program_id.clone(),
        slot_from: query.slot_from,
        slot_to: query.slot_to,
    };

    // Get summary stats for ETag
    let (total, max_slot, max_created_at) = repo.get_summary(&filter).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get summary");
        ApiError::Internal {
            reason: "Database query failed".to_string(),
        }
    })?;

    // Compute ETag
    let etag = compute_etag(
        &query,
        total,
        max_slot,
        max_created_at,
        &cache_config.etag_salt,
    );

    // Check If-None-Match
    if let Some(if_none_match) = req.headers().get("If-None-Match") {
        if let Ok(header_etag) = if_none_match.to_str() {
            if header_etag == etag {
                tracing::info!(etag = %etag, "ETag matched, returning 304");
                return Ok(HttpResponse::NotModified()
                    .insert_header(("ETag", etag.clone()))
                    .finish());
            }
        }
    }

    // Check cache
    let cache_key = format!("tx:list:{}", etag);
    if cache_config.enabled {
        if let Some(cached) = cache.get(&cache_key) {
            if cached.etag == etag {
                tracing::info!(etag = %etag, "Cache hit");
                return Ok(HttpResponse::Ok()
                    .insert_header(("ETag", etag.clone()))
                    .insert_header(("Content-Type", "application/json"))
                    .body(cached.data));
            }
        }
    }

    // Cache miss, query database
    tracing::info!(etag = %etag, "Cache miss, querying database");

    let pagination = Pagination {
        limit: query.limit as i64,
        offset: query.offset as i64,
    };

    let order_by_slot_desc = query.order == "desc";

    let items = repo
        .list(filter, pagination, order_by_slot_desc)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list transactions");
            ApiError::Internal {
                reason: "Database query failed".to_string(),
            }
        })?;

    let response = ListResponse {
        items,
        page: PageInfo {
            limit: query.limit,
            offset: query.offset,
            total,
        },
        sort: SortInfo {
            by: query.sort_by.clone(),
            order: query.order.clone(),
        },
    };

    let response_json = serde_json::to_vec(&response).map_err(|e| {
        tracing::error!(error = %e, "Failed to serialize response");
        ApiError::Internal {
            reason: "Serialization failed".to_string(),
        }
    })?;

    // Store in cache
    if cache_config.enabled {
        let cached = CachedResponse {
            data: response_json.clone(),
            etag: etag.clone(),
            cached_at: std::time::SystemTime::now(),
        };
        cache.set(&cache_key, cached, cache_config.ttl_secs);
    }

    Ok(HttpResponse::Ok()
        .insert_header(("ETag", etag))
        .insert_header(("Content-Type", "application/json"))
        .body(response_json))
}

// GET /api/transactions/{signature}
pub async fn get_transaction(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    let signature = path.into_inner();

    let pg_pool = app_state
        .postgres
        .as_ref()
        .ok_or_else(|| ApiError::ServiceUnavailable {
            details: "Database not available".to_string(),
        })?;

    let repo = TransactionRepository::new(pg_pool.clone());

    match repo.get_by_signature(&signature).await {
        Ok(Some(tx)) => Ok(HttpResponse::Ok().json(tx)),
        Ok(None) => Err(ApiError::NotFound {
            resource: "transaction".to_string(),
        }),
        Err(e) => {
            tracing::error!(error = %e, signature = %signature, "Failed to get transaction");
            Err(ApiError::Internal {
                reason: "Database query failed".to_string(),
            })
        }
    }
}

