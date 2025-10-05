use crate::models::{SolanaTransaction, AppError};
use sqlx::Row;
pub fn process_solana_transactions(rows: Vec<sqlx::postgres::PgRow>) -> Result<Vec<SolanaTransaction>, AppError> {
    let mut transactions = Vec::new();
    for row in rows {
        let transaction = SolanaTransaction {
            id: row.try_get("id").map_err(|e| AppError::SerializationError(e.to_string()))?,
            signature: row.try_get("signature").map_err(|e| AppError::SerializationError(e.to_string()))?,
            from_pubkey: row.try_get("from_pubkey").map_err(|e| AppError::SerializationError(e.to_string()))?,
            to_pubkey: row.try_get("to_pubkey").map_err(|e| AppError::SerializationError(e.to_string()))?,
            instructions: row.try_get("instructions").map_err(|e| AppError::SerializationError(e.to_string()))?,
            lamports: row.try_get("lamports").map_err(|e| AppError::SerializationError(e.to_string()))?,
            timestamp: row.try_get("timestamp").map_err(|e| AppError::SerializationError(e.to_string()))?,
            slot: row.try_get("slot").map_err(|e| AppError::SerializationError(e.to_string()))?,
        };
        transactions.push(transaction);
    }
    Ok(transactions)
}
