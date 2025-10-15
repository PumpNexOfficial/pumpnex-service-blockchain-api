mod app_state;
mod cache;
mod config;
mod errors;
mod http;
mod infra;
mod ingest;
mod metrics;
mod openapi;
mod repository;
mod ws;
mod telemetry;

use app_state::AppState;
use config::load_config;
use infra::{kafka, postgres, redis};
use ingest::kafka::start_kafka_ingestion;
use metrics::AppMetrics;
use telemetry::{init_telemetry, otel::shutdown_otel};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env file if exists
    let _ = dotenvy::dotenv();

    // Load configuration
    let config = load_config().unwrap_or_else(|e| {
        eprintln!("Failed to load configuration: {}", e);
        std::process::exit(1);
    });

    // Initialize metrics
    let metrics = AppMetrics::new().unwrap_or_else(|e| {
        eprintln!("Failed to initialize metrics: {}", e);
        std::process::exit(1);
    });

    // Initialize telemetry
    init_telemetry(&config.telemetry, &config.otel, &config.sentry);

    tracing::info!("Initializing integrations...");

    // Initialize integrations
    let pg_pool = postgres::init_postgres(&config.integrations, &config.db).await;
    let redis_conn = redis::init_redis(&config.integrations).await;
    let _kafka_client = kafka::init_kafka(&config.integrations).await;

    // Create AppState
    let app_state = AppState::new(
        config.service.clone(),
        pg_pool.clone(),
        redis_conn.clone(),
    );

    // Setup graceful shutdown
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl-c");
        tracing::info!("Shutdown signal received");
        let _ = tx.send(());
    });

    // Start Kafka ingestion if enabled
    if config.kafka.enabled {
        tracing::info!("Starting Kafka ingestion service");
        let kafka_config = config.kafka.clone();
        let ingest_config = config.ingest.clone();
        let app_state_clone = app_state.clone();
        
        tokio::spawn(async move {
            if let Err(e) = start_kafka_ingestion(kafka_config, ingest_config, app_state_clone).await {
                tracing::error!("Kafka ingestion failed: {}", e);
            }
        });
    } else {
        tracing::info!("Kafka ingestion disabled");
    }

    // Start HTTP server
    let server = http::start_server(config, app_state, metrics);
    
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                tracing::error!(error = %e, "Server error");
                return Err(e);
            }
        }
        _ = rx => {
            tracing::info!("Shutting down gracefully");
        }
    }

    // Cleanup integrations
    if let Some(pool) = pg_pool {
        tracing::info!("Closing PostgreSQL connection pool");
        pool.close().await;
    }

    // Shutdown OpenTelemetry
    shutdown_otel();

    tracing::info!("Shutdown complete");
    Ok(())
}
