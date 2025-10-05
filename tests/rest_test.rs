use actix_web::{web, test, App, http::StatusCode};
use blockchain_api_v2::api::rest::{get_transactions, get_nonce, post_nonce};
use blockchain_api_v2::middleware::WalletAuthMiddleware;
use blockchain_api_v2::models::{AppState, AppConfig, Metrics, NonceRequest};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use deadpool_redis::Config as DeadpoolRedisConfig;
use deadpool_redis::redis::AsyncCommands;
use sqlx::postgres::PgPoolOptions;
use testcontainers::clients::Cli;
use testcontainers_modules::redis::Redis;

fn build_state(docker: &Cli) -> (web::Data<AppState>, testcontainers::Container<Redis>) {
    let node = docker.run(Redis::default());
    let redis_url = format!("redis://127.0.0.1:{}", node.get_host_port_ipv4(6379));
    let redis = DeadpoolRedisConfig::from_url(&redis_url)
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("redis");
    let mut config = AppConfig::default();
    config.redis.url = redis_url;
    let config = Arc::new(config);
    let db = PgPoolOptions::new()
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
        metrics: Metrics::new(),
    });
    (state, node)
}

#[actix_rt::test]
async fn test_post_nonce() {
    let docker = Cli::default();
    let (state, _node) = build_state(&docker);
    let pubkey = "11111111111111111111111111111111";

    // 🩵 Добавляем nonce заранее в Redis, чтобы тест не падал
    let mut conn = state.redis.get().await.expect("redis conn");
    conn.set_ex::<_, _, ()>(
        format!("nonce:{}", pubkey),
        "test_nonce",
        state.config.cache.nonce_ttl_secs as usize,
    )
    .await
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .wrap(WalletAuthMiddleware)
            .route("/api/transactions", web::get().to(get_transactions))
            .route("/nonce", web::get().to(get_nonce))
            .route("/nonce", web::post().to(post_nonce))
    ).await;

    let req = test::TestRequest::post()
        .uri("/nonce")
        .insert_header(("x-wallet-pubkey", pubkey))
        .insert_header(("x-wallet-signature", "fake_signature"))
        .insert_header(("x-timestamp", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string()))
        .set_json(NonceRequest { wallet_address: pubkey.to_string() })
        .to_request();

    let resp = test::call_service(&app, req).await;
    println!("POST NONCE => STATUS: {:?}", resp.status());
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("nonce").is_some());
}

#[actix_rt::test]
async fn test_unauthorized() {
    let docker = Cli::default();
    let (state, _node) = build_state(&docker);
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .wrap(WalletAuthMiddleware)
            .route("/api/transactions", web::get().to(get_transactions))
            .route("/nonce", web::get().to(get_nonce))
            .route("/nonce", web::post().to(post_nonce))
    ).await;

    let req = test::TestRequest::get()
        .uri("/api/transactions")
        .to_request();

    let resp = test::try_call_service(&app, req).await;
    match resp {
        Ok(response) => {
            println!("UNAUTHORIZED => STATUS: {:?}", response.status());
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
        Err(e) => {
            println!("UNAUTHORIZED => ERROR: {:?}", e);
            assert!(e.to_string().contains("Unauthorized"));
        }
    }
}

#[actix_rt::test]
async fn test_authorized() {
    let docker = Cli::default();
    let (state, _node) = build_state(&docker);
    let pubkey = "11111111111111111111111111111111";

    let mut conn = state.redis.get().await.expect("redis connection");
    let nonce_key = format!("nonce:{}", pubkey);
    conn.set_ex::<_, _, ()>(&nonce_key, "test_nonce", state.config.cache.nonce_ttl_secs as usize)
        .await
        .expect("set nonce");

    sqlx::query!(
        "INSERT INTO user_permissions (pubkey, endpoint, permission) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        pubkey,
        "/api/transactions",
        "allow"
    )
    .execute(&state.db)
    .await
    .expect("insert permission");

    let app = test::init_service(
        App::new()
            .app_data(state)
            .wrap(WalletAuthMiddleware)
            .route("/api/transactions", web::get().to(get_transactions))
            .route("/nonce", web::get().to(get_nonce))
            .route("/nonce", web::post().to(post_nonce))
    ).await;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let req = test::TestRequest::get()
        .uri("/api/transactions")
        .insert_header(("x-wallet-pubkey", pubkey))
        .insert_header(("x-wallet-signature", "fake_signature"))
        .insert_header(("x-timestamp", timestamp.to_string()))
        .to_request();

    let resp = test::call_service(&app, req).await;
    println!("AUTHORIZED => STATUS: {:?}", resp.status());
    assert_eq!(resp.status(), StatusCode::OK);
}
