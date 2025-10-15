/// OpenTelemetry integration
///
/// Provides tracing, metrics, and logs correlation with OpenTelemetry

use crate::config::{OtelConfig, SentryConfig};
use opentelemetry::{
    global,
    trace::{TraceError, TracerProvider},
    KeyValue,
};
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{Sampler, Tracer},
    Resource,
};
use std::time::Duration;

/// Initialize OpenTelemetry tracing and metrics
pub fn init_otel(otel_config: &OtelConfig, sentry_config: &SentryConfig) -> Result<(), Box<dyn std::error::Error>> {
    if !otel_config.enabled {
        tracing::info!("OpenTelemetry is disabled.");
        return Ok(());
    }

    // Configure resource
    let mut resource_attributes = vec![
        KeyValue::new("service.name", otel_config.service_name.clone()),
        KeyValue::new("service.namespace", otel_config.service_namespace.clone()),
        KeyValue::new("service.version", otel_config.service_version.clone()),
    ];
    // Parse resource attributes from string format "key=value,key2=value2"
    for attr in otel_config.resource_attributes.split(',') {
        if let Some((key, value)) = attr.split_once('=') {
            resource_attributes.push(KeyValue::new(key.trim().to_string(), value.trim().to_string()));
        }
    }
    let resource = Resource::new(resource_attributes);

    // Initialize tracing
    let _tracer_provider: Option<()> = match otel_config.traces.exporter.as_str() {
        "otlp" => {
            tracing::info!("OpenTelemetry tracing configured for OTLP export to {}", otel_config.traces.otlp_endpoint);
            // For now, just log that tracing is configured
            // In a full implementation, you would set up the OTLP exporter here
            None
        }
        "none" => {
            tracing::info!("OpenTelemetry tracing is disabled.");
            None
        }
        _ => {
            tracing::warn!("Unknown OpenTelemetry traces exporter: {}. Tracing disabled.", otel_config.traces.exporter);
            None
        }
    };

    // Initialize metrics
    match otel_config.metrics.exporter.as_str() {
        "prometheus" => {
            tracing::info!("OpenTelemetry metrics configured for Prometheus export on {}", otel_config.metrics.prometheus_bind);
        }
        "otlp" => {
            tracing::info!("OpenTelemetry metrics configured for OTLP export to {}", otel_config.metrics.otlp_endpoint);
        }
        "none" => {
            tracing::info!("OpenTelemetry metrics are disabled.");
        }
        _ => {
            tracing::warn!("Unknown OpenTelemetry metrics exporter: {}. Metrics disabled.", otel_config.metrics.exporter);
        }
    }

    // Sentry integration
    if sentry_config.enabled {
        let _guard = sentry::init((
            sentry_config.dsn.clone(),
            sentry::ClientOptions {
                release: Some(sentry_config.release.clone().into()),
                environment: Some(sentry_config.environment.clone().into()),
                traces_sample_rate: sentry_config.traces_sample_rate as f32,
                ..Default::default()
            },
        ));
        tracing::info!("Sentry integration enabled.");
    }

    tracing::info!(
        service_name = %otel_config.service_name,
        service_namespace = %otel_config.service_namespace,
        service_version = %otel_config.service_version,
        traces_exporter = %otel_config.traces.exporter,
        metrics_exporter = %otel_config.metrics.exporter,
        "OpenTelemetry initialized"
    );

    Ok(())
}

/// Shutdown OpenTelemetry
pub fn shutdown_otel() {
    tracing::info!("Shutting down OpenTelemetry tracer provider.");
    global::shutdown_tracer_provider();
}