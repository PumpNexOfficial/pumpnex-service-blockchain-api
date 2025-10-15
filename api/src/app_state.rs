/// Application state

use crate::config::ServiceConfig;
use crate::infra::kafka::KafkaClient;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub service_config: ServiceConfig,
    pub postgres: Option<PgPool>,
    pub redis: Option<ConnectionManager>,
    // Kafka client is not included in AppState due to Clone limitations
}

// Kafka state managed separately (not in Clone-able AppState)
pub struct KafkaState {
    pub kafka: Option<KafkaClient>,
}

impl AppState {
    pub fn new(service_config: ServiceConfig, postgres: Option<PgPool>, redis: Option<ConnectionManager>) -> Self {
        Self {
            service_config,
            postgres,
            redis,
        }
    }
}

