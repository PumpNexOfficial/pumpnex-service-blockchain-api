use actix_web::{test, http::header, web, App, HttpResponse};
use blockchain_api_v2::middleware::WalletAuthMiddleware;
use blockchain_api_v2::models::{AppState, AppConfig, CacheConfig, Metrics, RateLimitConfig};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use deadpool_redis::Config as RedisConfig;
use deadpool_redis::redis::AsyncCommands;
use sqlx::PgPool;
use solana_sdk::signature::Signature;
use bs58;
use testcontainers::clients::Cli;
use testcontainers_modules::redis::Redis;

fn build_state(docker: &Cli) -> (web::Data<AppState>, testcontainers::Container<Redis>) {
    let node = docker.run(Redis::default());
    let redis_url = format!("redis://127.0.0.1:{}", node.get_host_port_ipv4(6379));
    let redis = RedisConfig::from_url(&redis_url)
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Failed to create redis pool");

    let state = web::Data::new(AppState {
        redis,
        db: PgPool::connect_lazy("postgres://postgres:postgres@localhost/test")
            .expect("Failed to create pg pool"),
        config: Arc::new(AppConfig {
            cache: CacheConfig {
                nonce_ttl_secs: 120,
                ..Default::default()
            },
            rate_limit: RateLimitConfig {
                ip_max_requests: 100,
                ip_window_secs: 60,
                user_max_requests: 1,
                user_window_secs: 60,
                admins_exempt: true,
            },
            ..Default::default()
        }),
        metrics: Metrics::new(),
    });

    (state, node)
}

#[actix_rt::test]
async fn test_wallet_auth_middleware_rejects_missing_headers() {
    let docker = Cli::default();
    let (state, _node) = build_state(&docker);

    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .wrap(WalletAuthMiddleware)
            .route("/ping", web::get().to(|| async { HttpResponse::Ok().body("pong") })),
    )
    .await;

    let req = test::TestRequest::get().uri("/ping").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
}

#[actix_rt::test]
async fn test_wallet_auth_middleware_rejects_expired_timestamp() {
    let docker = Cli::default();
    let (state, _node) = build_state(&docker);

    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .wrap(WalletAuthMiddleware)
            .route("/ping", web::get().to(|| async { HttpResponse::Ok().body("pong") })),
    )
    .await;

    let pubkey_str = "11111111111111111111111111111111";
    let old_ts = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        - 600)
        .to_string();

    let req = test::TestRequest::get()
        .uri("/ping")
        .insert_header((header::HeaderName::from_static("x-wallet-pubkey"), pubkey_str))
        .insert_header((header::HeaderName::from_static("x-wallet-signature"), "FAKE_SIG"))
        .insert_header((header::HeaderName::from_static("x-timestamp"), old_ts))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
}

#[actix_rt::test]
async fn test_wallet_auth_middleware_rejects_invalid_signature() {
    let docker = Cli::default();
    let (state, _node) = build_state(&docker);

    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .wrap(WalletAuthMiddleware)
            .route("/ping", web::get().to(|| async { HttpResponse::Ok().body("pong") })),
    )
    .await;

    let pubkey_str = "11111111111111111111111111111111";
    let signature = Signature::try_from(&[0u8; 64][..]).expect("Failed to create signature");
    let nonce = "test-nonce".to_string();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    let mut conn = state.redis.get().await.expect("Failed to get Redis connection");
    conn.set_ex::<_, _, ()>(&format!("nonce:{}", pubkey_str), &nonce, state.config.cache.nonce_ttl_secs)
        .await
        .unwrap();

    let req = test::TestRequest::get()
        .uri("/ping")
        .insert_header((header::HeaderName::from_static("x-wallet-pubkey"), pubkey_str))
        .insert_header((header::HeaderName::from_static("x-wallet-signature"), bs58::encode(signature.as_ref()).into_string()))
        .insert_header((header::HeaderName::from_static("x-timestamp"), timestamp))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
}

#[actix_rt::test]
async fn test_wallet_auth_middleware_rejects_rate_limit() {
    let docker = Cli::default();
    let (state, _node) = build_state(&docker);

    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .wrap(WalletAuthMiddleware)
            .route("/ping", web::get().to(|| async { HttpResponse::Ok().body("pong") })),
    )
    .await;

    let pubkey_str = "11111111111111111111111111111111";
    let signature = Signature::try_from(&[0u8; 64][..]).expect("Failed to create signature");
    let nonce = "test-nonce".to_string();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    let mut conn = state.redis.get().await.expect("Failed to get Redis connection");
    conn.set_ex::<_, _, ()>(&format!("nonce:{}", pubkey_str), &nonce, state.config.cache.nonce_ttl_secs)
        .await
        .unwrap();
    conn.incr::<_, i64, i64>(&format!("ratelimit:{}", pubkey_str), 2i64)
        .await
        .unwrap();

    let req = test::TestRequest::get()
        .uri("/ping")
        .insert_header((header::HeaderName::from_static("x-wallet-pubkey"), pubkey_str))
        .insert_header((header::HeaderName::from_static("x-wallet-signature"), bs58::encode(signature.as_ref()).into_string()))
        .insert_header((header::HeaderName::from_static("x-timestamp"), timestamp))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 429);
}
