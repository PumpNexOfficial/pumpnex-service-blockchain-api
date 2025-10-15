/// WebSocket bridge for fan-out events
///
/// Provides non-blocking communication between Kafka ingestion
/// and WebSocket layer for real-time transaction events.

use crate::ingest::{WsEvent, IngestStats};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

/// WebSocket event sender
pub type WsEventSender = mpsc::UnboundedSender<WsEvent>;

/// WebSocket event receiver
pub type WsEventReceiver = mpsc::UnboundedReceiver<WsEvent>;

/// WebSocket bridge for event distribution
pub struct WsBridge {
    sender: WsEventSender,
    stats: Arc<std::sync::Mutex<IngestStats>>,
}

impl WsBridge {
    /// Create new WebSocket bridge
    pub fn new() -> (Self, WsEventReceiver) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let stats = Arc::new(std::sync::Mutex::new(IngestStats::default()));
        
        let bridge = Self {
            sender,
            stats,
        };
        
        (bridge, receiver)
    }
    
    /// Send transaction event to WebSocket layer
    pub fn emit_transaction_event(&self, transaction: serde_json::Value) {
        let event = WsEvent {
            transaction,
            event_type: "transaction".to_string(),
        };
        
        match self.sender.send(event) {
            Ok(_) => {
                debug!("WebSocket event queued for distribution");
                if let Ok(mut stats) = self.stats.lock() {
                    stats.record_ws_event();
                }
            }
            Err(e) => {
                error!("Failed to send WebSocket event: {}", e);
                // Don't block ingestion on WS backpressure
            }
        }
    }
    
    /// Get current statistics
    pub fn get_stats(&self) -> IngestStats {
        if let Ok(stats) = self.stats.lock() {
            IngestStats {
                messages_received: stats.messages_received,
                messages_processed: stats.messages_processed,
                messages_inserted: stats.messages_inserted,
                messages_skipped: stats.messages_skipped,
                messages_failed: stats.messages_failed,
                dlq_messages_sent: stats.dlq_messages_sent,
                ws_events_emitted: stats.ws_events_emitted,
                last_processed_at: stats.last_processed_at,
            }
        } else {
            IngestStats::default()
        }
    }
}

impl Clone for WsBridge {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            stats: self.stats.clone(),
        }
    }
}

/// WebSocket event distributor
pub struct WsEventDistributor {
    receiver: WsEventReceiver,
    // This would connect to the actual WebSocket layer
    // For now, we'll just log the events
}

impl WsEventDistributor {
    /// Create new event distributor
    pub fn new(receiver: WsEventReceiver) -> Self {
        Self { receiver }
    }
    
    /// Start distributing events to WebSocket connections
    pub async fn start_distribution(&mut self) {
        debug!("Starting WebSocket event distribution");
        
        while let Some(event) = self.receiver.recv().await {
            debug!(
                "Distributing WebSocket event: type={}, signature={}",
                event.event_type,
                event.transaction.get("signature").and_then(|v| v.as_str()).unwrap_or("unknown")
            );
            
            // TODO: Integrate with actual WebSocket layer
            // For now, just log the event
            self.handle_event(event).await;
        }
    }
    
    /// Handle individual event
    async fn handle_event(&self, event: WsEvent) {
        // This is where we would:
        // 1. Get all active WebSocket connections
        // 2. Check if transaction matches their filters
        // 3. Send EVENT message to matching connections
        
        debug!("Handling WebSocket event: {:?}", event);
        
        // For now, just log the transaction
        if let Some(signature) = event.transaction.get("signature").and_then(|v| v.as_str()) {
            debug!("Transaction event for signature: {}", signature);
        }
    }
}
