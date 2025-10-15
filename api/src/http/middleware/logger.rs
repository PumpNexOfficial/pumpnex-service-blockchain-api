/// Logger middleware
///
/// Logs HTTP requests with structured fields
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
    time::Instant,
};

use super::request_id::RequestIdValue;

pub struct Logger;

impl<S, B> Transform<S, ServiceRequest> for Logger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LoggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(LoggerMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct LoggerMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for LoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = Instant::now();
        let method = req.method().to_string();
        let path = req.path().to_string();
        let remote_addr = req
            .connection_info()
            .peer_addr()
            .unwrap_or("unknown")
            .to_string();

        let request_id = req
            .extensions()
            .get::<RequestIdValue>()
            .map(|r| r.0.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let service = self.service.clone();

        Box::pin(async move {
            let res = service.call(req).await?;
            let duration_ms = start.elapsed().as_millis();
            let status = res.status().as_u16();

            tracing::info!(
                request_id = %request_id,
                method = %method,
                path = %path,
                status = %status,
                duration_ms = %duration_ms,
                remote_addr = %remote_addr,
                "HTTP request"
            );

            Ok(res)
        })
    }
}
