/// WebSocket module for real-time transaction streaming
///
/// Provides live feed of Solana transactions with filtering, rate limiting,
/// and subscription management.

pub mod tx;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    // Client messages
    Subscribe {
        filters: TransactionFilters,
        resume_from_slot: Option<i64>,
    },
    Unsubscribe {
        id: String,
    },
    Pong {
        ts: u64,
    },
    
    // Server messages
    Ack {
        id: String,
        filters: TransactionFilters,
    },
    Event {
        sub: String,
        tx: serde_json::Value, // SolanaTransaction
    },
    Error {
        code: String,
        message: String,
    },
    Ping {
        ts: u64,
    },
    Info {
        message: String,
    },
}

/// Transaction filters for subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionFilters {
    pub signature: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub program_id: Option<String>,
    pub slot_from: Option<i64>,
    pub slot_to: Option<i64>,
}

impl Default for TransactionFilters {
    fn default() -> Self {
        Self {
            signature: None,
            from: None,
            to: None,
            program_id: None,
            slot_from: None,
            slot_to: None,
        }
    }
}

/// Active subscription
#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: String,
    pub filters: TransactionFilters,
    pub created_at: std::time::Instant,
}

/// Connection state for rate limiting
#[derive(Debug)]
pub struct ConnectionState {
    pub subscriptions: HashMap<String, Subscription>,
    pub last_activity: std::time::Instant,
    pub client_msg_count: u32,
    pub client_msg_window_start: std::time::Instant,
    pub event_count: u32,
    pub event_window_start: std::time::Instant,
}

impl ConnectionState {
    pub fn new() -> Self {
        let now = std::time::Instant::now();
        Self {
            subscriptions: HashMap::new(),
            last_activity: now,
            client_msg_count: 0,
            client_msg_window_start: now,
            event_count: 0,
            event_window_start: now,
        }
    }
    
    pub fn update_activity(&mut self) {
        self.last_activity = std::time::Instant::now();
    }
    
    pub fn reset_client_msg_window(&mut self) {
        self.client_msg_count = 0;
        self.client_msg_window_start = std::time::Instant::now();
    }
    
    pub fn reset_event_window(&mut self) {
        self.event_count = 0;
        self.event_window_start = std::time::Instant::now();
    }
}

/// Generate unique subscription ID
pub fn generate_subscription_id() -> String {
    Uuid::new_v4().to_string()
}

/// Check if transaction matches filters
pub fn matches_filters(tx: &serde_json::Value, filters: &TransactionFilters) -> bool {
    // Helper to get string field from JSON
    let get_str = |field: &str| -> Option<String> {
        tx.get(field).and_then(|v| v.as_str()).map(|s| s.to_string())
    };
    
    // Helper to get i64 field from JSON
    let get_i64 = |field: &str| -> Option<i64> {
        tx.get(field).and_then(|v| v.as_i64())
    };
    
    // Check signature
    if let Some(ref sig) = filters.signature {
        if get_str("signature") != Some(sig.clone()) {
            return false;
        }
    }
    
    // Check from
    if let Some(ref from) = filters.from {
        if get_str("from_pubkey") != Some(from.clone()) {
            return false;
        }
    }
    
    // Check to
    if let Some(ref to) = filters.to {
        if get_str("to_pubkey") != Some(to.clone()) {
            return false;
        }
    }
    
    // Check program_id (in array)
    if let Some(ref program_id) = filters.program_id {
        if let Some(program_ids) = tx.get("program_ids").and_then(|v| v.as_array()) {
            if !program_ids.iter().any(|v| v.as_str() == Some(program_id)) {
                return false;
            }
        } else {
            return false;
        }
    }
    
    // Check slot_from
    if let Some(slot_from) = filters.slot_from {
        if let Some(slot) = get_i64("slot") {
            if slot < slot_from {
                return false;
            }
        }
    }
    
    // Check slot_to
    if let Some(slot_to) = filters.slot_to {
        if let Some(slot) = get_i64("slot") {
            if slot > slot_to {
                return false;
            }
        }
    }
    
    true
}
