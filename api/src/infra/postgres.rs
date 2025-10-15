/// PostgreSQL integration

use crate::config::{IntegrationsConfig, DbConfig};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

pub async fn init_postgres(config: &IntegrationsConfig, db_config: &DbConfig) -> Option<PgPool> {
    if !config.enable_postgres {
        tracing::info!("PostgreSQL integration disabled");
        return None;
    }

    if config.database_url.is_empty() {
        tracing::warn!("PostgreSQL enabled but database_url is empty");
        return None;
    }

    tracing::info!(
        database_url = %config.database_url.split('@').last().unwrap_or("***"),
        max_connections = %config.pg_max_connections,
        connect_timeout_ms = %config.pg_connect_timeout_ms,
        idle_timeout_ms = %config.pg_idle_timeout_ms,
        "Initializing PostgreSQL connection pool"
    );

    match PgPoolOptions::new()
        .max_connections(config.pg_max_connections)
        .acquire_timeout(Duration::from_millis(config.pg_connect_timeout_ms))
        .idle_timeout(Duration::from_millis(config.pg_idle_timeout_ms))
        .connect(&config.database_url)
        .await
    {
        Ok(pool) => {
            tracing::info!("PostgreSQL connection pool initialized successfully");
            
        // Run migrations if enabled
        if db_config.run_migrations_on_start {
            tracing::info!("Database migrations are enabled but not implemented yet");
            // TODO: Implement migrations when migration files are available
        }
            
            Some(pool)
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to initialize PostgreSQL connection pool");
            None
        }
    }
}

// TODO: Implement database migrations when migration files are available

pub async fn check_postgres_health(pool: &PgPool) -> Result<(), String> {
    match sqlx::query("SELECT 1")
        .fetch_one(pool)
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("PostgreSQL health check failed: {}", e)),
    }
}

