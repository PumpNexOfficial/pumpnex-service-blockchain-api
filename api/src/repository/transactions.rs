use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SolanaTransaction {
    pub signature: String,
    pub slot: i64,
    pub from_pubkey: Option<String>,
    pub to_pubkey: Option<String>,
    pub lamports: Option<i64>,
    pub program_ids: Option<Vec<String>>,
    pub instructions: serde_json::Value,
    pub block_time: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct NewTransaction {
    pub signature: String,
    pub slot: i64,
    pub from_pubkey: Option<String>,
    pub to_pubkey: Option<String>,
    pub lamports: Option<i64>,
    pub program_ids: Option<Vec<String>>,
    pub instructions: serde_json::Value,
    pub block_time: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct TransactionFilter {
    pub signature: Option<String>,
    pub from_pubkey: Option<String>,
    pub to_pubkey: Option<String>,
    pub program_id: Option<String>,
    pub slot_from: Option<i64>,
    pub slot_to: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Pagination {
    pub limit: i64,
    pub offset: i64,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            limit: 50,
            offset: 0,
        }
    }
}

pub struct TransactionRepository {
    pool: PgPool,
}

impl TransactionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get transaction by signature
    pub async fn get_by_signature(
        &self,
        signature: &str,
    ) -> Result<Option<SolanaTransaction>, sqlx::Error> {
        let tx = sqlx::query_as::<_, SolanaTransaction>(
            r#"
            SELECT signature, slot, from_pubkey, to_pubkey, lamports,
                   program_ids, instructions, block_time, created_at
            FROM solana_transactions
            WHERE signature = $1
            "#,
        )
        .bind(signature)
        .fetch_optional(&self.pool)
        .await?;

        Ok(tx)
    }

    /// Insert transaction or ignore if exists (idempotent)
    pub async fn insert_or_ignore(&self, tx: NewTransaction) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO solana_transactions (
                signature, slot, from_pubkey, to_pubkey, lamports,
                program_ids, instructions, block_time
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (signature) DO NOTHING
            "#,
        )
        .bind(&tx.signature)
        .bind(tx.slot)
        .bind(&tx.from_pubkey)
        .bind(&tx.to_pubkey)
        .bind(tx.lamports)
        .bind(&tx.program_ids)
        .bind(&tx.instructions)
        .bind(tx.block_time)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List transactions with filters, pagination, and ordering
    pub async fn list(
        &self,
        filter: TransactionFilter,
        pagination: Pagination,
        order_by_slot_desc: bool,
    ) -> Result<Vec<SolanaTransaction>, sqlx::Error> {
        let mut query = String::from(
            r#"
            SELECT signature, slot, from_pubkey, to_pubkey, lamports,
                   program_ids, instructions, block_time, created_at
            FROM solana_transactions
            WHERE 1=1
            "#,
        );

        let mut params: Vec<Box<dyn sqlx::Encode<sqlx::Postgres> + Send + Sync>> = Vec::new();
        let mut param_index = 1;

        // Build dynamic WHERE clauses
        if let Some(ref sig) = filter.signature {
            query.push_str(&format!(" AND signature = ${}", param_index));
            params.push(Box::new(sig.clone()));
            param_index += 1;
        }

        if let Some(ref from) = filter.from_pubkey {
            query.push_str(&format!(" AND from_pubkey = ${}", param_index));
            params.push(Box::new(from.clone()));
            param_index += 1;
        }

        if let Some(ref to) = filter.to_pubkey {
            query.push_str(&format!(" AND to_pubkey = ${}", param_index));
            params.push(Box::new(to.clone()));
            param_index += 1;
        }

        if let Some(ref program_id) = filter.program_id {
            query.push_str(&format!(" AND ${}::text = ANY(program_ids)", param_index));
            params.push(Box::new(program_id.clone()));
            param_index += 1;
        }

        if let Some(slot_from) = filter.slot_from {
            query.push_str(&format!(" AND slot >= ${}", param_index));
            params.push(Box::new(slot_from));
            param_index += 1;
        }

        if let Some(slot_to) = filter.slot_to {
            query.push_str(&format!(" AND slot <= ${}", param_index));
            params.push(Box::new(slot_to));
            param_index += 1;
        }

        // Order by
        if order_by_slot_desc {
            query.push_str(" ORDER BY slot DESC");
        } else {
            query.push_str(" ORDER BY slot ASC");
        }

        // Pagination
        query.push_str(&format!(
            " LIMIT ${} OFFSET ${}",
            param_index,
            param_index + 1
        ));
        params.push(Box::new(pagination.limit));
        params.push(Box::new(pagination.offset));

        // Execute query
        // Note: Dynamic query building with sqlx is complex, so for production
        // consider using typed query builders or conditional query construction
        // For now, use a simpler approach without dynamic params

        // Fallback: use basic query without dynamic filters for now
        let txs = if filter.signature.is_none()
            && filter.from_pubkey.is_none()
            && filter.to_pubkey.is_none()
            && filter.program_id.is_none()
            && filter.slot_from.is_none()
            && filter.slot_to.is_none()
        {
            // No filters, simple query
            let order = if order_by_slot_desc {
                "DESC"
            } else {
                "ASC"
            };
            sqlx::query_as::<_, SolanaTransaction>(&format!(
                r#"
                SELECT signature, slot, from_pubkey, to_pubkey, lamports,
                       program_ids, instructions, block_time, created_at
                FROM solana_transactions
                ORDER BY slot {}
                LIMIT $1 OFFSET $2
                "#,
                order
            ))
            .bind(pagination.limit)
            .bind(pagination.offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            // With filters - build conditionally
            self.list_with_filters(filter, pagination, order_by_slot_desc)
                .await?
        };

        Ok(txs)
    }

    async fn list_with_filters(
        &self,
        filter: TransactionFilter,
        pagination: Pagination,
        order_by_slot_desc: bool,
    ) -> Result<Vec<SolanaTransaction>, sqlx::Error> {
        // Build query with all possible filters
        let order = if order_by_slot_desc {
            "DESC"
        } else {
            "ASC"
        };

        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT signature, slot, from_pubkey, to_pubkey, lamports, program_ids, instructions, block_time, created_at FROM solana_transactions WHERE 1=1",
        );

        if let Some(ref sig) = filter.signature {
            query_builder.push(" AND signature = ");
            query_builder.push_bind(sig);
        }

        if let Some(ref from) = filter.from_pubkey {
            query_builder.push(" AND from_pubkey = ");
            query_builder.push_bind(from);
        }

        if let Some(ref to) = filter.to_pubkey {
            query_builder.push(" AND to_pubkey = ");
            query_builder.push_bind(to);
        }

        if let Some(ref program_id) = filter.program_id {
            query_builder.push(" AND ");
            query_builder.push_bind(program_id);
            query_builder.push(" = ANY(program_ids)");
        }

        if let Some(slot_from) = filter.slot_from {
            query_builder.push(" AND slot >= ");
            query_builder.push_bind(slot_from);
        }

        if let Some(slot_to) = filter.slot_to {
            query_builder.push(" AND slot <= ");
            query_builder.push_bind(slot_to);
        }

        query_builder.push(format!(" ORDER BY slot {} LIMIT ", order));
        query_builder.push_bind(pagination.limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(pagination.offset);

        let txs = query_builder
            .build_query_as::<SolanaTransaction>()
            .fetch_all(&self.pool)
            .await?;

        Ok(txs)
    }

    /// Get transactions since a specific slot (for WebSocket resume)
    pub async fn list_since_slot(
        &self,
        since_slot: i64,
        limit: i64,
    ) -> Result<Vec<SolanaTransaction>, sqlx::Error> {
        let transactions = sqlx::query_as::<_, SolanaTransaction>(
            "SELECT signature, slot, from_pubkey, to_pubkey, lamports, program_ids, instructions, block_time, created_at 
             FROM solana_transactions 
             WHERE slot > $1 
             ORDER BY slot ASC 
             LIMIT $2"
        )
        .bind(since_slot)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(transactions)
    }

    /// Bulk insert or ignore transactions (idempotent)
    pub async fn bulk_insert_or_ignore(
        &self,
        transactions: &[crate::ingest::NormalizedTransaction],
    ) -> Result<crate::ingest::BatchResult, sqlx::Error> {
        if transactions.is_empty() {
            return Ok(crate::ingest::BatchResult {
                processed: 0,
                inserted: 0,
                skipped: 0,
                errors: vec![],
            });
        }

        let mut inserted = 0;
        let mut skipped = 0;
        let mut errors = Vec::new();

        // Process transactions in chunks to avoid parameter limits
        const CHUNK_SIZE: usize = 50;
        for chunk in transactions.chunks(CHUNK_SIZE) {
            let mut query_builder = sqlx::QueryBuilder::new(
                "INSERT INTO solana_transactions (signature, slot, from_pubkey, to_pubkey, lamports, program_ids, instructions, block_time) VALUES "
            );

            let mut separated = query_builder.separated(", ");
            for tx in chunk {
                separated.push("(");
                separated.push_bind(&tx.signature);
                separated.push_bind(tx.slot);
                separated.push_bind(&tx.from_pubkey);
                separated.push_bind(&tx.to_pubkey);
                separated.push_bind(tx.lamports);
                separated.push_bind(&tx.program_ids);
                separated.push_bind(&tx.instructions);
                separated.push_bind(tx.block_time);
                separated.push(")");
            }

            query_builder.push(" ON CONFLICT (signature) DO NOTHING");

            match query_builder.build().execute(&self.pool).await {
                Ok(result) => {
                    let rows_affected = result.rows_affected();
                    inserted += rows_affected;
                    skipped += chunk.len() as u64 - rows_affected;
                }
                Err(e) => {
                    error!("Failed to insert chunk: {}", e);
                    for tx in chunk {
                        errors.push(crate::ingest::ProcessingError::DatabaseError {
                            signature: tx.signature.clone(),
                            error: e.to_string(),
                        });
                    }
                }
            }
        }

        Ok(crate::ingest::BatchResult {
            processed: transactions.len(),
            inserted: inserted as usize,
            skipped: skipped as usize,
            errors,
        })
    }

    /// Get summary statistics for ETag calculation
    pub async fn get_summary(
        &self,
        filter: &TransactionFilter,
    ) -> Result<(i64, i64, chrono::DateTime<chrono::Utc>), sqlx::Error> {
        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT COUNT(*) as total, COALESCE(MAX(slot), 0) as max_slot, COALESCE(MAX(created_at), '1970-01-01'::timestamptz) as max_created_at FROM solana_transactions WHERE 1=1",
        );

        if let Some(ref sig) = filter.signature {
            query_builder.push(" AND signature = ");
            query_builder.push_bind(sig);
        }

        if let Some(ref from) = filter.from_pubkey {
            query_builder.push(" AND from_pubkey = ");
            query_builder.push_bind(from);
        }

        if let Some(ref to) = filter.to_pubkey {
            query_builder.push(" AND to_pubkey = ");
            query_builder.push_bind(to);
        }

        if let Some(ref program_id) = filter.program_id {
            query_builder.push(" AND ");
            query_builder.push_bind(program_id);
            query_builder.push(" = ANY(program_ids)");
        }

        if let Some(slot_from) = filter.slot_from {
            query_builder.push(" AND slot >= ");
            query_builder.push_bind(slot_from);
        }

        if let Some(slot_to) = filter.slot_to {
            query_builder.push(" AND slot <= ");
            query_builder.push_bind(slot_to);
        }

        #[derive(sqlx::FromRow)]
        struct Summary {
            total: Option<i64>,
            max_slot: Option<i64>,
            max_created_at: Option<chrono::DateTime<chrono::Utc>>,
        }

        let summary = query_builder
            .build_query_as::<Summary>()
            .fetch_one(&self.pool)
            .await?;

        Ok((
            summary.total.unwrap_or(0),
            summary.max_slot.unwrap_or(0),
            summary
                .max_created_at
                .unwrap_or_else(|| chrono::DateTime::UNIX_EPOCH),
        ))
    }
}
