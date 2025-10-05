use actix::prelude::*;
use actix_web_actors::ws;
use tracing::info;
use crate::models::AppState;

pub struct WsSession {
    pub state: actix_web::web::Data<AppState>,
}

impl WsSession {
    pub fn new(state: actix_web::web::Data<AppState>) -> Self {
        WsSession { state }
    }
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("WebSocket session started");
        ctx.text("connected");
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        info!("WebSocket session stopped");
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                info!("Received WS text: {}", text);
                ctx.text(format!("echo: {}", text));
            }
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => {}
        }
    }
}
