//! Prometheus metrics collection for JSON-RPC

use prometheus::{
    CounterVec, Encoder, HistogramOpts, HistogramVec, IntGauge, Opts, Registry,
};
use std::sync::Arc;
use std::time::Duration;

/// Prometheus metrics collector for JSON-RPC
pub struct PrometheusMetrics {
    registry: Registry,
    request_counter: CounterVec,
    request_duration: HistogramVec,
    error_counter: CounterVec,
    active_connections: IntGauge,
}

impl PrometheusMetrics {
    /// Create a new Prometheus metrics collector
    pub fn new() -> Result<Self, prometheus::Error> {
        Self::with_prefix("jsonrpc")
    }

    /// Create a new metrics collector with custom prefix
    pub fn with_prefix(prefix: &str) -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let request_counter = CounterVec::new(
            Opts::new(
                format!("{}_requests_total", prefix),
                "Total number of JSON-RPC requests",
            ),
            &["method"],
        )?;

        let request_duration = HistogramVec::new(
            HistogramOpts::new(
                format!("{}_request_duration_seconds", prefix),
                "JSON-RPC request duration in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
            &["method"],
        )?;

        let error_counter = CounterVec::new(
            Opts::new(
                format!("{}_errors_total", prefix),
                "Total number of JSON-RPC errors",
            ),
            &["method"],
        )?;

        let active_connections = IntGauge::new(
            format!("{}_active_connections", prefix),
            "Number of active connections",
        )?;

        registry.register(Box::new(request_counter.clone()))?;
        registry.register(Box::new(request_duration.clone()))?;
        registry.register(Box::new(error_counter.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;

        Ok(Self {
            registry,
            request_counter,
            request_duration,
            error_counter,
            active_connections,
        })
    }

    /// Record a request with method, duration, and success status
    pub fn record_request(&self, method: &str, duration: Duration, success: bool) {
        // Limit cardinality by using a normalized method name
        let normalized_method = self.normalize_method(method);
        
        self.request_counter
            .with_label_values(&[normalized_method])
            .inc();

        self.request_duration
            .with_label_values(&[normalized_method])
            .observe(duration.as_secs_f64());

        if !success {
            self.error_counter
                .with_label_values(&[normalized_method])
                .inc();
        }
    }

    /// Increment active connections count
    pub fn connection_opened(&self) {
        self.active_connections.inc();
    }

    /// Decrement active connections count
    pub fn connection_closed(&self) {
        self.active_connections.dec();
    }

    /// Get the Prometheus registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Gather metrics in Prometheus text format
    pub fn gather_text(&self) -> Result<String, prometheus::Error> {
        use prometheus::TextEncoder;
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    /// Normalize method name to prevent cardinality explosion
    /// Keeps known methods as-is, groups unknown methods as "other"
    fn normalize_method<'a>(&self, method: &'a str) -> &'a str {
        // Common RPC methods - extend as needed
        const KNOWN_METHODS: &[&str] = &[
            "ping",
            "echo",
            "add",
            "subtract",
            "multiply",
            "divide",
            "healthcheck",
            "get_metrics",
            "get_health",
        ];

        if KNOWN_METHODS.contains(&method) {
            method
        } else {
            "other"
        }
    }
}

impl Default for PrometheusMetrics {
    fn default() -> Self {
        Self::new().expect("Failed to create default PrometheusMetrics")
    }
}

/// Builder for creating Prometheus metrics with custom configuration
pub struct PrometheusMetricsBuilder {
    prefix: String,
    known_methods: Vec<String>,
}

impl PrometheusMetricsBuilder {
    /// Create a new builder with default prefix
    pub fn new() -> Self {
        Self {
            prefix: "jsonrpc".to_string(),
            known_methods: vec![
                "ping".to_string(),
                "echo".to_string(),
                "healthcheck".to_string(),
            ],
        }
    }

    /// Set custom metric prefix
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Add known method names for cardinality control
    pub fn add_known_method(mut self, method: impl Into<String>) -> Self {
        self.known_methods.push(method.into());
        self
    }

    /// Build the metrics collector
    pub fn build(self) -> Result<PrometheusMetrics, prometheus::Error> {
        PrometheusMetrics::with_prefix(&self.prefix)
    }
}

impl Default for PrometheusMetricsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// RPC method handler that exposes metrics in Prometheus format
pub fn get_metrics_method(
    metrics: Arc<PrometheusMetrics>,
) -> impl Fn(Option<serde_json::Value>, Option<ash_rpc_core::RequestId>) -> ash_rpc_core::Response
{
    move |_params, id| {
        match metrics.gather_text() {
            Ok(text) => ash_rpc_core::rpc_success!(text, id),
            Err(e) => ash_rpc_core::rpc_error!(
                ash_rpc_core::error_codes::INTERNAL_ERROR,
                format!("Failed to gather metrics: {}", e),
                id
            ),
        }
    }
}

/// Enhanced health check that includes basic metrics
pub fn get_health_method(
    metrics: Arc<PrometheusMetrics>,
) -> impl Fn(Option<serde_json::Value>, Option<ash_rpc_core::RequestId>) -> ash_rpc_core::Response
{
    move |_params, id| {
        let health = serde_json::json!({
            "status": "ok",
            "active_connections": metrics.active_connections.get(),
        });
        ash_rpc_core::rpc_success!(health, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prometheus_metrics_creation() {
        let metrics = PrometheusMetrics::new().unwrap();
        assert!(metrics.registry().gather().len() > 0);
    }

    #[test]
    fn test_record_request() {
        let metrics = PrometheusMetrics::new().unwrap();
        metrics.record_request("ping", Duration::from_millis(10), true);
        metrics.record_request("echo", Duration::from_millis(20), false);
        
        let text = metrics.gather_text().unwrap();
        assert!(text.contains("jsonrpc_requests_total"));
        assert!(text.contains("jsonrpc_request_duration_seconds"));
        assert!(text.contains("jsonrpc_errors_total"));
    }

    #[test]
    fn test_connection_tracking() {
        let metrics = PrometheusMetrics::new().unwrap();
        assert_eq!(metrics.active_connections.get(), 0);
        
        metrics.connection_opened();
        assert_eq!(metrics.active_connections.get(), 1);
        
        metrics.connection_opened();
        assert_eq!(metrics.active_connections.get(), 2);
        
        metrics.connection_closed();
        assert_eq!(metrics.active_connections.get(), 1);
    }

    #[test]
    fn test_custom_prefix() {
        let metrics = PrometheusMetrics::with_prefix("custom").unwrap();
        let text = metrics.gather_text().unwrap();
        assert!(text.contains("custom_requests_total"));
    }
}
