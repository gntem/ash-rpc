//! Observability features for ash-rpc
//!
//! Provides metrics collection, distributed tracing, and unified observability wrapper.

use ash_rpc_core::{Message, MessageProcessor, ProcessorCapabilities, Response};
use async_trait::async_trait;
use std::sync::Arc;

#[cfg(feature = "prometheus")]
pub mod prometheus;

#[cfg(feature = "opentelemetry")]
pub mod tracing;

pub mod macros;

#[cfg(feature = "logging")]
use crate::logging::Logger;

/// Unified observability processor wrapping metrics, tracing, and logging
pub struct ObservableProcessor {
    inner: Arc<dyn MessageProcessor + Send + Sync>,
    #[cfg(feature = "prometheus")]
    metrics: Option<Arc<prometheus::PrometheusMetrics>>,
    #[cfg(feature = "opentelemetry")]
    tracer: Option<Arc<tracing::TracingProcessor>>,
    #[cfg(feature = "logging")]
    logger: Option<Arc<dyn Logger>>,
}

impl ObservableProcessor {
    /// Create a new builder for observable processor
    pub fn builder(processor: Arc<dyn MessageProcessor + Send + Sync>) -> ObservabilityBuilder {
        ObservabilityBuilder {
            processor,
            #[cfg(feature = "prometheus")]
            metrics: None,
            #[cfg(feature = "opentelemetry")]
            tracer: None,
            #[cfg(feature = "logging")]
            logger: None,
        }
    }
}

#[async_trait]
impl MessageProcessor for ObservableProcessor {
    async fn process_message(&self, message: Message) -> Option<Response> {
        #[cfg(feature = "logging")]
        if let Some(logger) = &self.logger {
            match &message {
                Message::Request(req) => {
                    logger.debug(
                        "Processing request",
                        &[("method", &req.method), ("has_id", &req.id.is_some())],
                    );
                }
                Message::Notification(notif) => {
                    logger.debug("Processing notification", &[("method", &notif.method)]);
                }
                Message::Response(_) => {
                    logger.debug("Received response", &[]);
                }
            }
        }

        #[cfg(feature = "prometheus")]
        let start = std::time::Instant::now();

        #[cfg(feature = "opentelemetry")]
        let span_guard = if let Some(tracer) = &self.tracer {
            tracer.start_span(&message)
        } else {
            None
        };

        let response = self.inner.process_message(message.clone()).await;

        #[cfg(feature = "prometheus")]
        if let Some(metrics) = &self.metrics {
            let duration = start.elapsed();
            let method = match &message {
                Message::Request(req) => &req.method,
                Message::Notification(notif) => &notif.method,
                Message::Response(_) => "response",
            };

            metrics.record_request(
                method,
                duration,
                response.as_ref().map(|r| r.is_success()).unwrap_or(true),
            );
        }

        #[cfg(feature = "opentelemetry")]
        if let Some(mut guard) = span_guard
            && let Some(response) = &response
            && !response.is_success()
        {
            guard.record_error();
        }

        #[cfg(feature = "logging")]
        if let Some(logger) = &self.logger
            && let Some(response) = &response
        {
            if response.is_success() {
                logger.debug("Request succeeded", &[]);
            } else {
                logger.warn("Request failed", &[]);
            }
        }

        response
    }

    fn get_capabilities(&self) -> ProcessorCapabilities {
        self.inner.get_capabilities()
    }
}

/// Builder for creating observable processors
pub struct ObservabilityBuilder {
    processor: Arc<dyn MessageProcessor + Send + Sync>,
    #[cfg(feature = "prometheus")]
    metrics: Option<Arc<prometheus::PrometheusMetrics>>,
    #[cfg(feature = "opentelemetry")]
    tracer: Option<Arc<tracing::TracingProcessor>>,
    #[cfg(feature = "logging")]
    logger: Option<Arc<dyn Logger>>,
}

impl ObservabilityBuilder {
    /// Add Prometheus metrics collection
    #[cfg(feature = "prometheus")]
    pub fn with_metrics(mut self, metrics: Arc<prometheus::PrometheusMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Add OpenTelemetry tracing
    #[cfg(feature = "opentelemetry")]
    pub fn with_tracing(mut self, tracer: Arc<tracing::TracingProcessor>) -> Self {
        self.tracer = Some(tracer);
        self
    }

    /// Add structured logging
    #[cfg(feature = "logging")]
    pub fn with_logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = Some(logger);
        self
    }

    /// Build the observable processor
    pub fn build(self) -> ObservableProcessor {
        ObservableProcessor {
            inner: self.processor,
            #[cfg(feature = "prometheus")]
            metrics: self.metrics,
            #[cfg(feature = "opentelemetry")]
            tracer: self.tracer,
            #[cfg(feature = "logging")]
            logger: self.logger,
        }
    }
}
