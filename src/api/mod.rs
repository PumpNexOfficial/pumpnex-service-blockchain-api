use actix_web::web;
use crate::auth;
mod rest;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            // публичные маршруты
            .service(auth::get_nonce)
            // защищённые маршруты
            .service(
                web::scope("")
                    .wrap(crate::middleware::WalletAuthMiddleware)
                    .service(rest::get_transactions)
            )
    );
}
