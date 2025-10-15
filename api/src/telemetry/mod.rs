/// Telemetry module
///
/// Initializes tracing/logging with structured output and OpenTelemetry integration

pub mod otel;

use crate::config::{OtelConfig, SentryConfig, TelemetryConfig};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_telemetry(config: &TelemetryConfig, otel_config: &OtelConfig, sentry_config: &SentryConfig) {
    // Initialize OpenTelemetry first if enabled
    if otel_config.enabled {
        if let Err(e) = otel::init_otel(otel_config, sentry_config) {
            tracing::error!("Failed to initialize OpenTelemetry: {}", e);
            // Fall back to basic telemetry
            init_basic_telemetry(config);
        }
    } else {
        init_basic_telemetry(config);
    }
}

fn init_basic_telemetry(config: &TelemetryConfig) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    let registry = tracing_subscriber::registry().with(env_filter);

    if config.log_format == "json" {
        registry
            .with(fmt::layer().json().flatten_event(true))
            .init();
    } else {
        registry.with(fmt::layer().compact()).init();
    }
}
