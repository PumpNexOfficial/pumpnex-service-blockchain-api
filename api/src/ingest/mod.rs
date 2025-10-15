/// Kafka ingestion module
///
/// Handles real-time transaction ingestion from Kafka topics,
/// normalization, idempotent database operations, and WebSocket fan-out.

pub mod kafka;
pub mod normalize;
pub mod bridge;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Raw transaction message from Kafka
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTransaction {
    pub signature: String,
    pub slot: i64,
    pub from: Option<String>,
    pub to: Option<String>,
    pub lamports: Option<i64>,
    pub program_ids: Option<Vec<String>>,
    pub instructions: Option<Vec<serde_json::Value>>,
    pub block_time: Option<String>,
}

/// Normalized transaction for database insertion
#[derive(Debug, Clone, serde::Serialize)]
pub struct NormalizedTransaction {
    pub signature: String,
    pub slot: i64,
    pub from_pubkey: Option<String>,
    pub to_pubkey: Option<String>,
    pub lamports: Option<i64>,
    pub program_ids: Option<Vec<String>>,
    pub instructions: serde_json::Value,
    pub block_time: Option<i64>,
}

/// Batch processing result
#[derive(Debug)]
pub struct BatchResult {
    pub processed: usize,
    pub inserted: usize,
    pub skipped: usize,
    pub errors: Vec<ProcessingError>,
}

/// Processing error types
#[derive(Debug, Clone)]
pub enum ProcessingError {
    ParseError {
        message: String,
        error: String,
    },
    ValidationError {
        field: String,
        reason: String,
    },
    DatabaseError {
        signature: String,
        error: String,
    },
    KafkaError {
        operation: String,
        error: String,
    },
}

/// DLQ message for failed processing
#[derive(Debug, Serialize)]
pub struct DlqMessage {
    pub original_message: serde_json::Value,
    pub error: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub retry_count: u32,
}

/// WebSocket event for fan-out
#[derive(Debug, Clone)]
pub struct WsEvent {
    pub transaction: serde_json::Value,
    pub event_type: String,
}

/// Ingest statistics
#[derive(Debug, Default)]
pub struct IngestStats {
    pub messages_received: u64,
    pub messages_processed: u64,
    pub messages_inserted: u64,
    pub messages_skipped: u64,
    pub messages_failed: u64,
    pub dlq_messages_sent: u64,
    pub ws_events_emitted: u64,
    pub last_processed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl IngestStats {
    pub fn record_message_received(&mut self) {
        self.messages_received += 1;
    }
    
    pub fn record_message_processed(&mut self) {
        self.messages_processed += 1;
        self.last_processed_at = Some(chrono::Utc::now());
    }
    
    pub fn record_message_inserted(&mut self) {
        self.messages_inserted += 1;
    }
    
    pub fn record_message_skipped(&mut self) {
        self.messages_skipped += 1;
    }
    
    pub fn record_message_failed(&mut self) {
        self.messages_failed += 1;
    }
    
    pub fn record_dlq_sent(&mut self) {
        self.dlq_messages_sent += 1;
    }
    
    pub fn record_ws_event(&mut self) {
        self.ws_events_emitted += 1;
    }
}
