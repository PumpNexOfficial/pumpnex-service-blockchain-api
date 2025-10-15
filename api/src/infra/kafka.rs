/// Kafka integration

use crate::config::IntegrationsConfig;
use rdkafka::client::DefaultClientContext;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use std::time::Duration;

pub struct KafkaClient {
    pub consumer: BaseConsumer,
    pub metadata_timeout: Duration,
}

pub async fn init_kafka(config: &IntegrationsConfig) -> Option<KafkaClient> {
    if !config.enable_kafka {
        tracing::info!("Kafka integration disabled");
        return None;
    }

    if config.kafka_brokers.is_empty() {
        tracing::warn!("Kafka enabled but kafka_brokers is empty");
        return None;
    }

    tracing::info!(
        brokers = %config.kafka_brokers,
        client_id = %config.kafka_client_id,
        metadata_timeout_ms = %config.kafka_metadata_timeout_ms,
        "Initializing Kafka client"
    );

    match ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("client.id", &config.kafka_client_id)
        .set("group.id", format!("{}-health", config.kafka_client_id))
        .set("enable.auto.commit", "false")
        .create::<BaseConsumer>()
    {
        Ok(consumer) => {
            tracing::info!("Kafka client initialized successfully");
            Some(KafkaClient {
                consumer,
                metadata_timeout: Duration::from_millis(config.kafka_metadata_timeout_ms),
            })
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to create Kafka client");
            None
        }
    }
}

pub fn check_kafka_health(client: &KafkaClient) -> Result<(), String> {
    match client
        .consumer
        .fetch_metadata(None, client.metadata_timeout)
    {
        Ok(metadata) => {
            let broker_count = metadata.brokers().len();
            if broker_count > 0 {
                Ok(())
            } else {
                Err("No Kafka brokers available".to_string())
            }
        }
        Err(e) => Err(format!("Kafka health check failed: {}", e)),
    }
}

