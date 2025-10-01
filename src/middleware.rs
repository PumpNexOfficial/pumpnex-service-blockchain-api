use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error as ActixError,
    body::{BoxBody, MessageBody},
};
use std::clone::Clone;
use futures_util::future::{ready, LocalBoxFuture, Ready};
use tracing::info;

pub struct WalletAuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for WalletAuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError> + 'static + Clone,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = ActixError;
    type InitError = ();
    type Transform = WalletAuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(WalletAuthMiddlewareService { service }))
    }
}

pub struct WalletAuthMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for WalletAuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError> + 'static + Clone,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = ActixError;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().clone();
        let uri = req.uri().clone();
        let peer = req.peer_addr().map(|p| p.to_string());
        let service = self.service.clone();
        Box::pin(async move {
            info!("Request: {} {} from {}", method, uri, peer.as_ref().map_or("unknown", |p| p.as_str()));
            let res = service.call(req).await?;
            Ok(res.map_into_boxed_body())
        })
    }
}
