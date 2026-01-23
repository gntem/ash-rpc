//! Contributed JSON-RPC methods and utilities for ash-rpc

pub mod transports;

#[cfg(feature = "healthcheck")]
pub mod healthcheck;

#[cfg(feature = "tower")]
pub mod middleware;

#[cfg(any(feature = "logging", feature = "prometheus", feature = "opentelemetry"))]
pub mod observability;

// Re-export transport modules for convenience
#[cfg(feature = "axum")]
pub use transports::axum;

// Re-export healthcheck for convenience
#[cfg(feature = "healthcheck")]
pub use healthcheck::*;

// Re-export tower middleware for convenience
#[cfg(feature = "tower")]
pub use middleware::*;

// Re-export tower when feature is enabled
#[cfg(feature = "tower")]
pub use tower;

// Re-export logger types from core
pub use ash_rpc_core::logger::{LogKv, Logger, NoopLogger, StdoutLogger, TracingLogger};

// Re-export observability for convenience
#[cfg(any(feature = "logging", feature = "prometheus", feature = "opentelemetry"))]
pub use observability::{ObservabilityBuilder, ObservableProcessor};

// Re-export prometheus types
#[cfg(feature = "prometheus")]
pub use observability::prometheus::{PrometheusMetrics, PrometheusMetricsBuilder};

// Re-export prometheus crate
#[cfg(feature = "prometheus")]
pub use prometheus;

// Re-export OpenTelemetry types
#[cfg(feature = "opentelemetry")]
pub use observability::tracing::{SpanGuard, TracingBuilder, TracingProcessor};

// Re-export OpenTelemetry crates
#[cfg(feature = "opentelemetry")]
pub use opentelemetry;

#[cfg(feature = "opentelemetry")]
pub use opentelemetry_otlp;

#[cfg(feature = "opentelemetry")]
pub use opentelemetry_sdk;

// Re-export audit logging types from core
#[cfg(feature = "audit-logging")]
pub use ash_rpc_core::audit_logging::{
    AuditBackend, AuditEvent, AuditEventBuilder, AuditEventType, AuditIntegrity, AuditProcessor,
    AuditProcessorBuilder, AuditResult, AuditSeverity, ChecksumIntegrity, CombinedIntegrity,
    MultiAuditBackend, NoIntegrity, NoopAuditBackend, SequenceIntegrity, StderrAuditBackend,
    StdoutAuditBackend, log_auth_event, log_security_violation,
};
