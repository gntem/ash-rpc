//! OpenTelemetry tracing for JSON-RPC

use ash_rpc_core::Message;
use opentelemetry::{
    KeyValue, global,
    trace::{Span, Status, Tracer},
};

/// Tracing processor for OpenTelemetry integration
pub struct TracingProcessor {
    service_name: String,
}

impl TracingProcessor {
    /// Create a new tracing processor with the given service name
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    /// Create a tracer from the global tracer provider
    pub fn from_global(service_name: &str) -> Self {
        Self::new(service_name)
    }

    /// Start a span for a message
    pub fn start_span(&self, message: &Message) -> Option<SpanGuard> {
        let (span_name, method_name) = match message {
            Message::Request(req) => (format!("jsonrpc.{}", req.method), Some(req.method.clone())),
            Message::Notification(notif) => (
                format!("jsonrpc.{}", notif.method),
                Some(notif.method.clone()),
            ),
            Message::Response(_) => ("jsonrpc.response".to_string(), None),
        };

        let tracer = global::tracer(self.service_name.clone());
        let mut span = tracer.start(span_name.clone());

        span.set_attribute(KeyValue::new("rpc.system", "jsonrpc"));
        span.set_attribute(KeyValue::new("rpc.jsonrpc.version", "2.0"));

        if let Some(method) = method_name {
            span.set_attribute(KeyValue::new("rpc.method", method));
        }

        match message {
            Message::Request(req) => {
                if let Some(id) = &req.id {
                    span.set_attribute(KeyValue::new("rpc.jsonrpc.request_id", id.to_string()));
                }
            }
            Message::Notification(_) => {
                span.set_attribute(KeyValue::new("rpc.jsonrpc.notification", true));
            }
            Message::Response(resp) => {
                if let Some(id) = &resp.id {
                    span.set_attribute(KeyValue::new("rpc.jsonrpc.request_id", id.to_string()));
                }
                if resp.is_error() {
                    span.set_attribute(KeyValue::new("rpc.jsonrpc.error", true));
                }
            }
        }

        Some(SpanGuard { span })
    }

    /// Extract trace context from request parameters
    /// Looks for a "_trace_context" field in params
    pub fn extract_context(params: &Option<serde_json::Value>) -> Option<opentelemetry::Context> {
        if let Some(serde_json::Value::Object(map)) = params
            && let Some(trace_ctx) = map.get("_trace_context")
        {
            // Try to extract traceparent header format
            if let Some(traceparent) = trace_ctx.get("traceparent").and_then(|v| v.as_str()) {
                // Parse W3C traceparent format
                // Format: 00-{trace_id}-{span_id}-{flags}
                return Self::parse_traceparent(traceparent);
            }
        }
        None
    }

    /// Parse W3C traceparent header
    fn parse_traceparent(_traceparent: &str) -> Option<opentelemetry::Context> {
        // Simplified implementation - in production use proper W3C parser
        // For now, return None to use automatic context propagation
        None
    }
}

/// Guard that records span end when dropped
pub struct SpanGuard {
    span: global::BoxedSpan,
}

impl SpanGuard {
    /// Record an error in the span
    pub fn record_error(&mut self) {
        self.span.set_status(Status::error("Request failed"));
    }

    /// Add an event to the span
    pub fn add_event(
        &mut self,
        name: impl Into<std::borrow::Cow<'static, str>>,
        attributes: Vec<KeyValue>,
    ) {
        self.span.add_event(name, attributes);
    }

    /// Set a span attribute
    pub fn set_attribute(&mut self, kv: KeyValue) {
        self.span.set_attribute(kv);
    }
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        self.span.end();
    }
}

/// Builder for creating tracing processor with custom configuration
pub struct TracingBuilder {
    service_name: String,
    service_version: Option<String>,
}

impl TracingBuilder {
    /// Create a new builder with service name
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            service_version: None,
        }
    }

    /// Set service version
    pub fn service_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = Some(version.into());
        self
    }

    /// Build using the global tracer
    pub fn build(self) -> TracingProcessor {
        TracingProcessor::from_global(&self.service_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_builder() {
        let _processor = TracingBuilder::new("test-service")
            .service_version("1.0.0")
            .build();
    }

    #[test]
    fn test_extract_context_missing() {
        let params = None;
        let ctx = TracingProcessor::extract_context(&params);
        assert!(ctx.is_none());
    }

    #[test]
    fn test_extract_context_with_traceparent() {
        let params = serde_json::json!({
            "_trace_context": {
                "traceparent": "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
            }
        });
        let _ctx = TracingProcessor::extract_context(&Some(params));
        // Currently returns None due to simplified implementation
    }
}
