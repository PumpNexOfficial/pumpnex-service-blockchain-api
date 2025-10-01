use actix_web::{App, HttpServer, middleware::Compress, web, HttpResponse};
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::fmt;
use config::{Config, File as ConfigFile, Environment};
use sqlx::postgres::PgPoolOptions;
use dotenvy::dotenv;
use serde_json::json;
use crate::models::{AppState, AppConfig, AppError};
mod middleware;
mod models;
mod handlers;
mod routes;

fn load_rustls_config(app_config: &AppConfig) -> ServerConfig {
    let cert_file = &mut BufReader::new(File::open(&app_config.server.tls_cert_path).expect("cannot open cert.pem"));
    let key_file = &mut BufReader::new(File::open(&app_config.server.tls_key_path).expect("cannot open key.pem"));
    let cert_chain: Vec<rustls::Certificate> = certs(cert_file)
        .unwrap()
        .into_iter()
        .map(rustls::Certificate)
        .collect();
    let mut keys: Vec<rustls::PrivateKey> = pkcs8_private_keys(key_file)
        .unwrap()
        .into_iter()
        .map(rustls::PrivateKey)
        .collect();
    if keys.is_empty() {
        panic!("No private keys found in key.pem");
    }
    ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, keys.remove(0))
        .expect("bad certificate/key")
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    fmt().with_env_filter("info").init();
    info!("Starting Blockchain API...");
    let config = Config::builder()
        .add_source(ConfigFile::with_name("config/default").required(true))
        .add_source(Environment::with_prefix("APP").separator("__"))
        .build()?;
    let app_config: AppConfig = config.try_deserialize()?;
    let app_config_arc = Arc::new(app_config);
    info!("Connecting to Redis...");
    let redis_url = std::env::var("APP_REDIS_URL").unwrap_or(app_config_arc.redis.url.clone());
    let redis_pool = deadpool_redis::Config::from_url(&redis_url)
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .map_err(|e| AppError::StartupError(format!("Redis pool creation failed: {}", e)))?;
    info!("Connecting to PostgreSQL...");
    let db_url = std::env::var("APP_DATABASE_URL").unwrap_or(app_config_arc.database.url.clone());
    let db = PgPoolOptions::new()
        .max_connections(app_config_arc.database.max_connections)
        .acquire_timeout(std::time::Duration::from_secs(app_config_arc.database.acquire_timeout_secs))
        .idle_timeout(std::time::Duration::from_secs(app_config_arc.database.idle_timeout_secs))
        .connect(&db_url)
        .await
        .map_err(|e| AppError::StartupError(format!("Database connection failed: {}", e)))?;
    if app_config_arc.database.run_migrations {
        info!("Running database migrations...");
        sqlx::migrate!("./migrations")
            .run(&db)
            .await
            .map_err(|e| AppError::StartupError(format!("Database migration failed: {}", e)))?;
        info!("Database migrations applied.");
    }
    let state = web::Data::new(AppState {
        redis: redis_pool,
        db,
        config: app_config_arc.clone(),
    });
    let bind_config = app_config_arc.clone();
    info!("Starting server on {}:{}", bind_config.server.host, bind_config.server.port);
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(Compress::default())
            // Публичные маршруты без middleware
            .service(web::resource("/health").route(web::get().to(|| async { HttpResponse::Ok().json(json!({"status":"ok"})) })))
            .service(routes::auth::get_nonce)
            // Защищённые маршруты с WalletAuthMiddleware (в будущей итерации)
            .service(
                web::scope("/api/transactions")
                    .service(handlers::transactions::get_transactions)
            )
    })
    .bind(format!("{}:{}", bind_config.server.host, bind_config.server.port))?
    .run()
    .await?;
    Ok(())
}
