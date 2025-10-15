/// Route modules

pub mod admin_waf;
pub mod auth;
pub mod health;
pub mod metrics;
pub mod openapi_routes;
pub mod transactions;
pub mod version;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg
        .route("/healthz", web::get().to(health::healthz))
        .route("/readyz", web::get().to(health::readyz))
        .route("/version", web::get().to(version::version))
        .route("/metrics", web::get().to(metrics::get_metrics))
        .route("/swagger-ui/", web::get().to(openapi_routes::swagger_ui))
        .service(
            web::scope("/api-docs")
                .route("/openapi.json", web::get().to(openapi_routes::openapi_json)),
        )
        .service(
            web::scope("/api")
                .service(
                    web::scope("/auth")
                        .route("/nonce", web::post().to(auth::get_nonce))
                )
                .service(
                    web::scope("/transactions")
                        .route("", web::get().to(transactions::list_transactions))
                        .route("/{signature}", web::get().to(transactions::get_transaction))
                )
        );
}
