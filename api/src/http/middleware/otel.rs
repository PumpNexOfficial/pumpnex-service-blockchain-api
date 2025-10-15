use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::{self, LocalBoxFuture, Ready};
use std::{rc::Rc, time::Instant};

#[derive(Clone)]
pub struct OtelMiddleware;

impl OtelMiddleware {
    pub fn new() -> Self {
        OtelMiddleware
    }
}

impl<S, B> Transform<S, ServiceRequest> for OtelMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = OtelService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ready(Ok(OtelService {
            service: Rc::new(service),
        }))
    }
}

pub struct OtelService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for OtelService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let start_time = Instant::now();

        Box::pin(async move {
            let res = service.call(req).await;
            let duration = start_time.elapsed();
            
            // Log request with timing
            tracing::info!(
                duration_ms = duration.as_millis(),
                "HTTP request processed"
            );
            
            res
        })
    }
}