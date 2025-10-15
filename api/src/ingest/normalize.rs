/// Message normalization and validation
///
/// Converts raw Kafka messages to normalized database format
/// with validation and error handling.

use crate::ingest::{NormalizedTransaction, ProcessingError, RawTransaction};
use serde_json;
use tracing::{debug, error, warn};

/// Normalize raw transaction message
pub fn normalize_transaction(raw: &RawTransaction) -> Result<NormalizedTransaction, ProcessingError> {
    // Validate signature
    if raw.signature.is_empty() {
        return Err(ProcessingError::ValidationError {
            field: "signature".to_string(),
            reason: "Signature cannot be empty".to_string(),
        });
    }
    
    // Validate slot
    if raw.slot < 0 {
        return Err(ProcessingError::ValidationError {
            field: "slot".to_string(),
            reason: "Slot must be non-negative".to_string(),
        });
    }
    
    // Validate program_ids array size
    if let Some(ref program_ids) = raw.program_ids {
        if program_ids.len() > 50 {
            return Err(ProcessingError::ValidationError {
                field: "program_ids".to_string(),
                reason: "Too many program IDs (max 50)".to_string(),
            });
        }
    }
    
    // Validate instructions array size
    if let Some(ref instructions) = raw.instructions {
        if instructions.len() > 100 {
            return Err(ProcessingError::ValidationError {
                field: "instructions".to_string(),
                reason: "Too many instructions (max 100)".to_string(),
            });
        }
    }
    
    // Parse block_time
    let block_time = if let Some(ref time_str) = raw.block_time {
        match chrono::DateTime::parse_from_rfc3339(time_str) {
            Ok(dt) => Some(dt.timestamp()),
            Err(e) => {
                warn!("Failed to parse block_time: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    // Normalize instructions to JSON
    let instructions = match &raw.instructions {
        Some(inst) => serde_json::Value::Array(inst.clone()),
        None => serde_json::Value::Array(vec![]),
    };
    
    Ok(NormalizedTransaction {
        signature: raw.signature.clone(),
        slot: raw.slot,
        from_pubkey: raw.from.clone(),
        to_pubkey: raw.to.clone(),
        lamports: raw.lamports,
        program_ids: raw.program_ids.clone(),
        instructions,
        block_time,
    })
}

/// Parse JSON message from Kafka
pub fn parse_raw_message(payload: &[u8]) -> Result<RawTransaction, ProcessingError> {
    // Check message size
    if payload.len() > 1_048_576 {
        return Err(ProcessingError::ValidationError {
            field: "message_size".to_string(),
            reason: "Message too large (max 1MB)".to_string(),
        });
    }
    
    // Parse JSON
    let raw: RawTransaction = match serde_json::from_slice(payload) {
        Ok(parsed) => parsed,
        Err(e) => {
            error!("Failed to parse JSON message: {}", e);
            return Err(ProcessingError::ParseError {
                message: String::from_utf8_lossy(payload).to_string(),
                error: e.to_string(),
            });
        }
    };
    
    debug!("Parsed raw transaction: signature={}, slot={}", raw.signature, raw.slot);
    Ok(raw)
}

/// Validate normalized transaction before database insertion
pub fn validate_normalized(tx: &NormalizedTransaction) -> Result<(), ProcessingError> {
    // Check signature format (basic base58 check)
    if tx.signature.len() < 80 || tx.signature.len() > 100 {
        return Err(ProcessingError::ValidationError {
            field: "signature".to_string(),
            reason: "Invalid signature length".to_string(),
        });
    }
    
    // Check pubkey formats if present
    if let Some(ref from) = tx.from_pubkey {
        if from.len() != 44 {
            return Err(ProcessingError::ValidationError {
                field: "from_pubkey".to_string(),
                reason: "Invalid pubkey length".to_string(),
            });
        }
    }
    
    if let Some(ref to) = tx.to_pubkey {
        if to.len() != 44 {
            return Err(ProcessingError::ValidationError {
                field: "to_pubkey".to_string(),
                reason: "Invalid pubkey length".to_string(),
            });
        }
    }
    
    // Check lamports if present
    if let Some(lamports) = tx.lamports {
        if lamports < 0 {
            return Err(ProcessingError::ValidationError {
                field: "lamports".to_string(),
                reason: "Lamports must be non-negative".to_string(),
            });
        }
    }
    
    Ok(())
}
