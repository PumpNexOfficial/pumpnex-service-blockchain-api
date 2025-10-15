/// Request ID middleware
///
/// Extracts or generates request ID and adds it to response headers
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
};
use uuid::Uuid;

pub struct RequestId {
    header_name: String,
}

impl RequestId {
    pub fn new(header_name: String) -> Self {
        Self { header_name }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RequestId
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestIdMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestIdMiddleware {
            service: Rc::new(service),
            header_name: self.header_name.clone(),
        }))
    }
}

pub struct RequestIdMiddleware<S> {
    service: Rc<S>,
    header_name: String,
}

impl<S, B> Service<ServiceRequest> for RequestIdMiddleware<S>
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
        let request_id = req
            .headers()
            .get(&self.header_name)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        req.extensions_mut()
            .insert(RequestIdValue(request_id.clone()));

        let service = self.service.clone();
        let header_name = self.header_name.clone();

        Box::pin(async move {
            let mut res = service.call(req).await?;
            res.headers_mut().insert(
                actix_web::http::header::HeaderName::from_bytes(header_name.as_bytes()).unwrap(),
                actix_web::http::header::HeaderValue::from_str(&request_id).unwrap(),
            );
            Ok(res)
        })
    }
}

#[derive(Clone)]
pub struct RequestIdValue(pub String);
