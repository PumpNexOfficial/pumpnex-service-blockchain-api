use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpRequest, HttpResponse,
};
use futures_util::future::{self, LocalBoxFuture, Ready};
use std::{rc::Rc, time::Instant};

use crate::config::SecurityConfig;

#[derive(Clone)]
pub struct SecurityHeadersMiddleware {
    config: SecurityConfig,
}

impl SecurityHeadersMiddleware {
    pub fn new(config: SecurityConfig) -> Self {
        Self { config }
    }
}

impl<S, B> Transform<S, ServiceRequest> for SecurityHeadersMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SecurityHeadersService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ready(Ok(SecurityHeadersService {
            service: Rc::new(service),
            config: self.config.clone(),
        }))
    }
}

pub struct SecurityHeadersService<S> {
    service: Rc<S>,
    config: SecurityConfig,
}

impl<S, B> Service<ServiceRequest> for SecurityHeadersService<S>
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
        let config = self.config.clone();
        let start_time = Instant::now();

        Box::pin(async move {
            let res = service.call(req).await;
            let duration = start_time.elapsed();

            match res {
                Ok(mut response) => {
                    let headers = response.headers_mut();

                    // HSTS (HTTP Strict Transport Security)
                    if config.hsts_enabled {
                        headers.insert(
                            actix_web::http::header::STRICT_TRANSPORT_SECURITY,
                            format!("max-age={}", config.hsts_max_age_secs).parse().unwrap(),
                        );
                    }

                    // X-Frame-Options
                    headers.insert(
                        actix_web::http::header::HeaderName::from_static("x-frame-options"),
                        config.frame_options.parse().unwrap(),
                    );

                    // X-Content-Type-Options
                    headers.insert(
                        actix_web::http::header::HeaderName::from_static("x-content-type-options"),
                        config.x_content_type_options.parse().unwrap(),
                    );

                    // Referrer-Policy
                    headers.insert(
                        actix_web::http::header::HeaderName::from_static("referrer-policy"),
                        config.referrer_policy.parse().unwrap(),
                    );

                    // Permissions-Policy
                    headers.insert(
                        actix_web::http::header::HeaderName::from_static("permissions-policy"),
                        config.permissions_policy.parse().unwrap(),
                    );

                    // Content-Security-Policy (if enabled)
                    if config.csp_enabled {
                        headers.insert(
                            actix_web::http::header::HeaderName::from_static("content-security-policy"),
                            config.csp.parse().unwrap(),
                        );
                    }

                    // Log security headers applied
                    tracing::debug!(
                        hsts_enabled = %config.hsts_enabled,
                        frame_options = %config.frame_options,
                        csp_enabled = %config.csp_enabled,
                        duration_ms = duration.as_millis(),
                        "Security headers applied"
                    );

                    Ok(response)
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        duration_ms = duration.as_millis(),
                        "Security headers middleware error"
                    );
                    Err(e)
                }
            }
        })
    }
}
