use actix_web::{web, HttpResponse, Error as ActixError, HttpRequest};
use actix_web_actors::ws;
use tracing::info;
use crate::models::{AppError, AppState};
use crate::websocket::WsSession;

pub async fn websocket(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ActixError> {
    let pubkey_str = match req.headers().get("X-Wallet-Pubkey") {
        Some(val) => val
            .to_str()
            .map_err(|_| AppError::BadRequest("Invalid pubkey header".to_string()))?
            .to_string(),
        None => return Err(AppError::Unauthorized("Missing X-Wallet-Pubkey".to_string()).into()),
    };

    info!("Starting WebSocket connection for pubkey: {}", pubkey_str);
    ws::start(WsSession::new(state), &req, stream)
}
