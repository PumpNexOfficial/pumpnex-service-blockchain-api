use actix_web::{web, App, HttpServer};
use blockchain_api_v2::middleware::WalletAuthMiddleware;
use blockchain_api_v2::api::rest::{get_transactions, post_nonce};
use blockchain_api_v2::routes::auth::get_nonce;
use blockchain_api_v2::models::{AppState, AppConfig};
use std::sync::Arc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = Arc::new(AppConfig::default());
    let redis = deadpool_redis::Config::from_url(&config.redis.url)
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("redis");

    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .acquire_timeout(std::time::Duration::from_secs(config.database.acquire_timeout_secs))
        .idle_timeout(std::time::Duration::from_secs(config.database.idle_timeout_secs))
        .connect_lazy_with(sqlx::postgres::PgConnectOptions::new()
            .host("localhost")
            .port(5433)
            .username("postgres")
            .password("postgres")
            .database("test"));

    let state = web::Data::new(AppState {
        redis,
        db,
        config,
        metrics: blockchain_api_v2::models::Metrics::new(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(WalletAuthMiddleware)
            .route("/api/transactions", web::get().to(get_transactions))
            .route("/nonce", web::get().to(get_nonce))
            .route("/nonce", web::post().to(post_nonce))
    })
    .bind("0.0.0.0:8081")?
    .run()
    .await
}
