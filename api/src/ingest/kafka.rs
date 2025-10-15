/// Kafka consumer for transaction ingestion
///
/// Handles message polling, batch processing, and error handling
/// with DLQ support and WebSocket fan-out.

use crate::{
    app_state::AppState,
    config::{KafkaConfig, IngestConfig},
    ingest::{
        normalize::{normalize_transaction, parse_raw_message, validate_normalized},
        bridge::{WsBridge, WsEventDistributor},
        BatchResult, DlqMessage, IngestStats, NormalizedTransaction, ProcessingError, RawTransaction,
    },
    repository::transactions::TransactionRepository,
};
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    producer::{FutureProducer, FutureRecord},
    ClientConfig, Message,
};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Kafka ingestion service
pub struct KafkaIngestion {
    consumer: StreamConsumer,
    producer: FutureProducer,
    config: KafkaConfig,
    ingest_config: IngestConfig,
    app_state: AppState,
    ws_bridge: WsBridge,
}

impl KafkaIngestion {
    /// Create new Kafka ingestion service
    pub async fn new(
        config: KafkaConfig,
        ingest_config: IngestConfig,
        app_state: AppState,
        ws_bridge: WsBridge,
    ) -> Result<Self, String> {
        // Create consumer
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .set("group.id", &config.group_id)
            .set("enable.auto.commit", "false")
            .set("session.timeout.ms", &config.session_timeout_ms.to_string())
            .set("max.poll.records", &config.max_poll_records.to_string())
            .set("message.max.bytes", &config.message_max_bytes.to_string())
            .create()
            .map_err(|e| format!("Failed to create Kafka consumer: {}", e))?;

        // Create producer for DLQ
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .create()
            .map_err(|e| format!("Failed to create Kafka producer: {}", e))?;

        // Subscribe to input topic
        consumer
            .subscribe(&[&config.input_topic])
            .map_err(|e| format!("Failed to subscribe to topic {}: {}", config.input_topic, e))?;

        info!(
            "Kafka ingestion initialized: brokers={}, group_id={}, input_topic={}",
            config.brokers, config.group_id, config.input_topic
        );

        Ok(Self {
            consumer,
            producer,
            config,
            ingest_config,
            app_state,
            ws_bridge,
        })
    }

    /// Start the ingestion loop
    pub async fn run(&mut self) -> Result<(), String> {
        info!("Starting Kafka ingestion loop");
        
        let mut batch = Vec::new();
        let mut last_poll = std::time::Instant::now();
        
        loop {
            // Poll for messages
            match self.consumer.recv().await {
                Ok(message) => {
                    debug!("Received message from partition {}", message.partition());
                    
                    // Process message
                    match self.process_message(&message).await {
                        Ok(Some(normalized)) => {
                            batch.push(normalized);
                            
                            // Process batch if full or timeout reached
                            if batch.len() >= self.ingest_config.db_insert_batch_size
                                || last_poll.elapsed() >= Duration::from_millis(self.config.poll_interval_ms)
                            {
                                self.process_batch(&mut batch).await;
                                last_poll = std::time::Instant::now();
                            }
                        }
                        Ok(None) => {
                            // Message skipped (duplicate, etc.)
                        }
                        Err(e) => {
                            error!("Failed to process message: {:?}", e);
                            self.send_to_dlq(&message, &e).await;
                        }
                    }
                }
                Err(e) => {
                    error!("Kafka consumer error: {}", e);
                    sleep(Duration::from_millis(self.config.retry_backoff_ms)).await;
                }
            }
        }
    }

    /// Process individual message
    async fn process_message(
        &self,
        message: &rdkafka::message::BorrowedMessage<'_>,
    ) -> Result<Option<NormalizedTransaction>, ProcessingError> {
        let payload = message.payload().ok_or_else(|| {
            ProcessingError::ParseError {
                message: "Empty message payload".to_string(),
                error: "No payload".to_string(),
            }
        })?;

        // Parse raw message
        let raw = parse_raw_message(payload)?;
        
        // Normalize transaction
        let normalized = normalize_transaction(&raw)?;
        
        // Validate normalized transaction
        validate_normalized(&normalized)?;
        
        debug!("Processed transaction: signature={}", normalized.signature);
        Ok(Some(normalized))
    }

    /// Process batch of transactions
    async fn process_batch(&self, batch: &mut Vec<NormalizedTransaction>) {
        if batch.is_empty() {
            return;
        }

        let batch_size = batch.len();
        debug!("Processing batch of {} transactions", batch_size);

        // Get database connection
        let pg_pool = match &self.app_state.postgres {
            Some(pool) => pool,
            None => {
                error!("PostgreSQL not available for batch processing");
                return;
            }
        };

        let repo = TransactionRepository::new(pg_pool.clone());
        
        // Process batch with retries
        let mut retry_count = 0;
        let processed_batch = batch.clone();
        
        while retry_count < self.config.max_retries {
            match repo.bulk_insert_or_ignore(&processed_batch).await {
                Ok(result) => {
                    info!(
                        "Batch processed: total={}, inserted={}, skipped={}",
                        result.processed, result.inserted, result.skipped
                    );
                    
                    // Emit WebSocket events for inserted transactions
                    if self.ingest_config.emit_ws_events {
                        for tx in &processed_batch[..result.inserted] {
                            let tx_json = serde_json::to_value(tx).unwrap_or_default();
                            self.ws_bridge.emit_transaction_event(tx_json);
                        }
                    }
                    
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    error!("Batch processing failed (attempt {}): {}", retry_count, e);
                    
                    if retry_count >= self.config.max_retries {
                        error!("Max retries exceeded for batch processing");
                        // Send to DLQ
                        for tx in &processed_batch {
                            self.send_transaction_to_dlq(tx, &e.to_string()).await;
                        }
                        break;
                    }
                    
                    // Wait before retry
                    sleep(Duration::from_millis(self.config.retry_backoff_ms * retry_count as u64)).await;
                }
            }
        }
        
        batch.clear();
    }

    /// Send message to DLQ
    async fn send_to_dlq(&self, message: &rdkafka::message::BorrowedMessage<'_>, error: &ProcessingError) {
        let dlq_message = DlqMessage {
            original_message: serde_json::Value::String(
                String::from_utf8_lossy(message.payload().unwrap_or(&[])).to_string()
            ),
            error: format!("{:?}", error),
            timestamp: chrono::Utc::now(),
            retry_count: 0,
        };

        let dlq_payload = match serde_json::to_vec(&dlq_message) {
            Ok(payload) => payload,
            Err(e) => {
                error!("Failed to serialize DLQ message: {}", e);
                return;
            }
        };

        let key = format!("dlq-{}", chrono::Utc::now().timestamp());
        let record = FutureRecord::to(&self.config.dlq_topic)
            .payload(&dlq_payload)
            .key(&key);

        match self.producer.send(record, Duration::from_secs(5)).await {
            Ok(_) => {
                debug!("Message sent to DLQ: {}", self.config.dlq_topic);
            }
            Err((e, _)) => {
                error!("Failed to send message to DLQ: {}", e);
            }
        }
    }

    /// Send transaction to DLQ
    async fn send_transaction_to_dlq(&self, tx: &NormalizedTransaction, error: &str) {
        let dlq_message = DlqMessage {
            original_message: serde_json::to_value(tx).unwrap_or_default(),
            error: error.to_string(),
            timestamp: chrono::Utc::now(),
            retry_count: 0,
        };

        let dlq_payload = match serde_json::to_vec(&dlq_message) {
            Ok(payload) => payload,
            Err(e) => {
                error!("Failed to serialize transaction DLQ message: {}", e);
                return;
            }
        };

        let record = FutureRecord::to(&self.config.dlq_topic)
            .payload(&dlq_payload)
            .key(&tx.signature);

        match self.producer.send(record, Duration::from_secs(5)).await {
            Ok(_) => {
                debug!("Transaction sent to DLQ: {}", tx.signature);
            }
            Err((e, _)) => {
                error!("Failed to send transaction to DLQ: {}", e);
            }
        }
    }
}

/// Start Kafka ingestion service
pub async fn start_kafka_ingestion(
    config: KafkaConfig,
    ingest_config: IngestConfig,
    app_state: AppState,
) -> Result<(), String> {
    if !config.enabled {
        info!("Kafka ingestion disabled");
        return Ok(());
    }

    // Create WebSocket bridge
    let (ws_bridge, ws_receiver) = WsBridge::new();
    
    // Start WebSocket event distributor
    let mut distributor = WsEventDistributor::new(ws_receiver);
    tokio::spawn(async move {
        distributor.start_distribution().await;
    });

    // Create and start Kafka ingestion
    let mut ingestion = KafkaIngestion::new(config, ingest_config, app_state, ws_bridge).await?;
    
    info!("Starting Kafka ingestion service");
    ingestion.run().await
}
