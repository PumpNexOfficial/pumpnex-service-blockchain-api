/// HTTP server module

pub mod middleware;
pub mod routes;

use actix_cors::Cors;
use actix_web::{http, web, App, HttpServer};
use std::io;
use rustls::{ServerConfig, pki_types::{CertificateDer, PrivateKeyDer}};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;

use crate::app_state::AppState;
use crate::cache;
use crate::config::Config;
use crate::metrics::AppMetrics;
use crate::ws::tx::tx_websocket;
use middleware::{logger::Logger, otel::OtelMiddleware, ratelimit::RateLimit, request_id::RequestId, security_headers::SecurityHeadersMiddleware, wallet_auth::WalletAuth, waf::WafMiddleware};
use std::sync::Arc;

/// Load TLS certificates from files
fn load_tls_config(cert_path: &str, key_path: &str) -> io::Result<ServerConfig> {
    // Load certificate
    let cert_file = File::open(cert_path)?;
    let mut cert_reader = BufReader::new(cert_file);
    let cert_chain: Vec<CertificateDer> = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    // Load private key
    let key_file = File::open(key_path)?;
    let mut key_reader = BufReader::new(key_file);
    let mut keys: Vec<PrivateKeyDer> = pkcs8_private_keys(&mut key_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
        .into_iter()
        .map(PrivateKeyDer::Pkcs8)
        .collect();
    
    if keys.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "No private key found"));
    }

    let key = keys.remove(0);

    // Create TLS config
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    Ok(config)
}

pub async fn start_server(config: Config, app_state: AppState, metrics: AppMetrics) -> io::Result<()> {
    let bind_addr = format!("{}:{}", config.server.host, config.server.port);
    
    tracing::info!(
        service_name = %config.service.name,
        service_version = %config.service.version,
        bind_addr = %bind_addr,
        tls_enabled = %config.server.tls_enabled,
        log_level = %config.telemetry.log_level,
        log_format = %config.telemetry.log_format,
        "Starting HTTP server"
    );

    let app_state = web::Data::new(app_state);
    let auth_config = web::Data::new(config.auth.clone());
    let cache_config = web::Data::new(config.cache.clone());
    let cache = web::Data::new(cache::create_cache(
        &config.cache.backend,
        config.cache.max_entries,
    ));
    let ws_config = web::Data::new(config.ws.clone());
    let kafka_config = web::Data::new(config.kafka.clone());
    let metrics_data = web::Data::new(metrics);
    let request_id_header = config.telemetry.request_id_header.clone();
    let rate_limiter = RateLimit::new(config.rate_limit.clone());
    let wallet_auth = WalletAuth::new(config.auth.clone());
    let waf_middleware = WafMiddleware::new(config.waf.clone(), Some(Arc::new(app_state.get_ref().clone()))).unwrap_or_else(|e| {
        tracing::error!("Failed to initialize WAF middleware: {}", e);
        std::process::exit(1);
    });
    let otel_middleware = OtelMiddleware::new();
    let security_headers = SecurityHeadersMiddleware::new(config.security.clone());
    let cors_origins = config.security.cors_allowed_origins.clone();
    let cors_methods = config.security.cors_allowed_methods.clone();
    let cors_headers = config.security.cors_allowed_headers.clone();
    let body_limit = config.server.request_body_limit_bytes;

    // TODO: Implement TLS support later
    tracing::info!("Starting HTTP server (no TLS)");
    HttpServer::new(move || {
        // Configure CORS inside the closure
        let mut cors = Cors::default();
        for origin in &cors_origins {
            if origin == "*" {
                cors = cors.allow_any_origin();
                break;
            } else {
                cors = cors.allowed_origin(origin);
            }
        }
        
        for method_str in &cors_methods {
            let method = match method_str.as_str() {
                "GET" => http::Method::GET,
                "POST" => http::Method::POST,
                "PUT" => http::Method::PUT,
                "PATCH" => http::Method::PATCH,
                "DELETE" => http::Method::DELETE,
                "OPTIONS" => http::Method::OPTIONS,
                _ => continue,
            };
            cors = cors.allowed_methods(vec![method]);
        }
        
        if cors_headers.contains(&"*".to_string()) {
            cors = cors.allow_any_header();
        } else {
            cors = cors.allowed_headers(
                cors_headers
                    .iter()
                    .filter_map(|h| h.parse::<http::header::HeaderName>().ok())
                    .collect::<Vec<_>>(),
            );
        }

        App::new()
            .app_data(app_state.clone())
            .app_data(auth_config.clone())
            .app_data(cache_config.clone())
            .app_data(cache.clone())
            .app_data(ws_config.clone())
            .app_data(kafka_config.clone())
            .app_data(metrics_data.clone())
            .app_data(web::PayloadConfig::new(body_limit))
            .wrap(cors)
            .wrap(otel_middleware.clone())
            .wrap(Logger)
            .wrap(wallet_auth.clone())
            .wrap(rate_limiter.clone())
            .wrap(waf_middleware.clone())
            .wrap(security_headers.clone())
            .wrap(RequestId::new(request_id_header.clone()))
            .configure(|cfg| routes::configure(cfg))
            .route(&config.ws.path, web::get().to(tx_websocket))
    })
    .bind(&bind_addr)?
    .run()
    .await
}
