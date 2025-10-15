/// WebSocket transaction streaming endpoint
///
/// Handles real-time transaction events with filtering, rate limiting,
/// and subscription management.

use crate::{
    app_state::AppState,
    config::WsConfig,
    ws::{ConnectionState, Subscription, TransactionFilters, WsMessage, generate_subscription_id},
};
use actix_web::{
    web::{Data, Payload},
    Error, HttpRequest, HttpResponse,
};
use actix_web_actors::ws;
use actix_web_actors::ws::{Message, ProtocolError, WebsocketContext};
use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use serde_json;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// WebSocket actor for transaction streaming
pub struct TxWebSocket {
    pub state: ConnectionState,
    pub config: WsConfig,
    pub app_state: AppState,
    pub last_ping: Instant,
}

impl TxWebSocket {
    pub fn new(config: WsConfig, app_state: AppState) -> Self {
        Self {
            state: ConnectionState::new(),
            config,
            app_state,
            last_ping: Instant::now(),
        }
    }
    
    /// Handle incoming WebSocket message
    fn handle_message(&mut self, msg: &str, ctx: &mut WebsocketContext<Self>) {
        self.state.update_activity();
        
        // Parse JSON message
        let ws_msg: WsMessage = match serde_json::from_str(msg) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Invalid JSON message: {}", e);
                self.send_error(ctx, "invalid_message", "Invalid JSON format");
                return;
            }
        };
        
        // Check rate limits for client messages
        if !self.check_client_rate_limit(ctx) {
            return;
        }
        
        match ws_msg {
            WsMessage::Subscribe { filters, resume_from_slot } => {
                self.handle_subscribe(filters, resume_from_slot, ctx);
            }
            WsMessage::Unsubscribe { id } => {
                self.handle_unsubscribe(&id, ctx);
            }
            WsMessage::Pong { ts: _ } => {
                debug!("Received pong from client");
                self.last_ping = Instant::now();
            }
            _ => {
                warn!("Unexpected message type from client");
                self.send_error(ctx, "invalid_message", "Unexpected message type");
            }
        }
    }
    
    /// Handle subscription request
    fn handle_subscribe(&mut self, filters: TransactionFilters, resume_from_slot: Option<i64>, ctx: &mut WebsocketContext<Self>) {
        // Check subscription limit
        if self.state.subscriptions.len() >= self.config.max_subscriptions_per_conn as usize {
            self.send_error(ctx, "too_many_subscriptions", "Maximum subscriptions exceeded");
            return;
        }
        
        let sub_id = generate_subscription_id();
        let subscription = Subscription {
            id: sub_id.clone(),
            filters: filters.clone(),
            created_at: Instant::now(),
        };
        
        self.state.subscriptions.insert(sub_id.clone(), subscription);
        
        // Send ACK
        let ack = WsMessage::Ack {
            id: sub_id.clone(),
            filters,
        };
        self.send_message(ctx, &ack);
        
        // Handle resume from slot if specified
        if let Some(slot) = resume_from_slot {
            // Send info message about resume
            let info = WsMessage::Info {
                message: format!("Resuming from slot {}", slot),
            };
            self.send_message(ctx, &info);
        }
        
        info!("Client subscribed with {} filters", self.state.subscriptions.len());
    }
    
    /// Handle unsubscription request
    fn handle_unsubscribe(&mut self, id: &str, _ctx: &mut WebsocketContext<Self>) {
        if self.state.subscriptions.remove(id).is_some() {
            debug!("Client unsubscribed from {}", id);
        } else {
            warn!("Client tried to unsubscribe from unknown subscription: {}", id);
        }
    }
    
    /// Check client message rate limit
    fn check_client_rate_limit(&mut self, ctx: &mut WebsocketContext<Self>) -> bool {
        let now = Instant::now();
        let window_duration = Duration::from_secs(60);
        
        // Reset window if needed
        if now.duration_since(self.state.client_msg_window_start) >= window_duration {
            self.state.reset_client_msg_window();
        }
        
        self.state.client_msg_count += 1;
        
        if self.state.client_msg_count > self.config.max_client_msg_per_min {
            self.send_error(ctx, "rate_limited", "Too many client messages");
            ctx.close(Some(ws::CloseCode::Policy.into()));
            return false;
        }
        
        true
    }
    
    /// Check event rate limit
    fn check_event_rate_limit(&mut self) -> bool {
        let now = Instant::now();
        let window_duration = Duration::from_secs(1);
        
        // Reset window if needed
        if now.duration_since(self.state.event_window_start) >= window_duration {
            self.state.reset_event_window();
        }
        
        self.state.event_count += 1;
        self.state.event_count <= self.config.max_events_per_sec
    }
    
    /// Send error message
    fn send_error(&self, ctx: &mut WebsocketContext<Self>, code: &str, message: &str) {
        let error = WsMessage::Error {
            code: code.to_string(),
            message: message.to_string(),
        };
        self.send_message(ctx, &error);
    }
    
    /// Send WebSocket message
    fn send_message(&self, ctx: &mut WebsocketContext<Self>, msg: &WsMessage) {
        match serde_json::to_string(msg) {
            Ok(json) => {
                ctx.text(json);
            }
            Err(e) => {
                error!("Failed to serialize message: {}", e);
            }
        }
    }
    
    /// Send ping to client
    fn send_ping(&mut self, ctx: &mut WebsocketContext<Self>) {
        let ping = WsMessage::Ping {
            ts: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        self.send_message(ctx, &ping);
        self.last_ping = Instant::now();
    }
    
    /// Check if connection is idle
    fn is_idle(&self) -> bool {
        Instant::now().duration_since(self.state.last_activity) >= Duration::from_secs(self.config.idle_timeout_secs)
    }
    
    /// Check if ping is overdue
    fn is_ping_overdue(&self) -> bool {
        Instant::now().duration_since(self.last_ping) >= Duration::from_secs(self.config.ping_interval_secs)
    }
}

impl Actor for TxWebSocket {
    type Context = WebsocketContext<Self>;
    
    fn started(&mut self, ctx: &mut Self::Context) {
        info!("WebSocket connection established");
        
        // Start ping timer
        ctx.run_interval(Duration::from_secs(self.config.ping_interval_secs), |act, ctx| {
            if act.is_ping_overdue() {
                act.send_ping(ctx);
            }
        });
        
        // Start idle timeout check
        ctx.run_interval(Duration::from_secs(10), |act, ctx| {
            if act.is_idle() {
                info!("Closing idle WebSocket connection");
                ctx.close(Some(ws::CloseCode::Normal.into()));
            }
        });
    }
    
    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("WebSocket connection closed");
    }
}

impl StreamHandler<Result<Message, ProtocolError>> for TxWebSocket {
    fn handle(&mut self, msg: Result<Message, ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(Message::Text(text)) => {
                self.handle_message(&text, ctx);
            }
            Ok(Message::Close(reason)) => {
                debug!("WebSocket closed: {:?}", reason);
                ctx.stop();
            }
            Ok(Message::Ping(payload)) => {
                ctx.pong(&payload);
            }
            Ok(Message::Pong(_)) => {
                // Client responded to ping
                self.last_ping = Instant::now();
            }
            Err(e) => {
                error!("WebSocket protocol error: {}", e);
                ctx.stop();
            }
            _ => {}
        }
    }
}

/// WebSocket endpoint handler
pub async fn tx_websocket(
    req: HttpRequest,
    stream: Payload,
    config: Data<WsConfig>,
    app_state: Data<AppState>,
) -> Result<HttpResponse, Error> {
    if !config.enabled {
        return Ok(HttpResponse::NotFound().finish());
    }
    
    let ws = TxWebSocket::new(config.get_ref().clone(), app_state.get_ref().clone());
    let resp = ws::start(ws, &req, stream)?;
    Ok(resp)
}
