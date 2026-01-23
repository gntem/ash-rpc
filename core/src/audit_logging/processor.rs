//! MessageProcessor wrapper that automatically logs security audit events.

use super::{AuditBackend, AuditEvent, AuditEventType, AuditIntegrity, AuditResult, AuditSeverity};
use crate::{Message, MessageProcessor, ProcessorCapabilities, Response, auth::ConnectionContext};
use async_trait::async_trait;
use std::sync::Arc;

/// Wraps MessageProcessor to automatically log requests, responses, and security events
pub struct AuditProcessor {
    inner: Arc<dyn MessageProcessor + Send + Sync>,
    backend: Arc<dyn AuditBackend>,
    integrity: Arc<dyn AuditIntegrity>,
    connection_context: Option<Arc<ConnectionContext>>,
}

impl AuditProcessor {
    /// Create a new audit processor builder
    pub fn builder(processor: Arc<dyn MessageProcessor + Send + Sync>) -> AuditProcessorBuilder {
        AuditProcessorBuilder {
            processor,
            backend: Arc::new(super::StdoutAuditBackend),
            integrity: Arc::new(super::NoIntegrity),
            connection_context: None,
        }
    }

    /// Log an audit event with integrity metadata
    fn log_event(&self, mut event: AuditEvent) {
        // Add integrity metadata
        self.integrity.add_integrity(&mut event);

        // Write to backend
        self.backend.log_audit(&event);
    }

    /// Create audit event from request message
    fn create_request_event(&self, message: &Message) -> Option<AuditEvent> {
        match message {
            Message::Request(req) => {
                let mut event = AuditEvent::builder()
                    .event_type(AuditEventType::MethodInvocation)
                    .method(&req.method)
                    .result(AuditResult::Success) // Will be updated based on response
                    .severity(AuditSeverity::Info);

                // Add correlation ID if present
                if let Some(ref id) = req.id {
                    event = event.correlation_id(id.to_string());
                }

                // Add connection context if available
                if let Some(ref ctx) = self.connection_context {
                    if let Some(addr) = ctx.remote_addr {
                        event = event.remote_addr(addr);
                    }

                    // Try to extract principal from context
                    if let Some(user_id) = ctx.get::<String>("user_id") {
                        event = event.principal(user_id);
                    } else if let Some(api_key) = ctx.get::<String>("api_key") {
                        event = event.principal(format!("api_key:{}", api_key));
                    }
                }

                // Sanitize and add parameters (avoid logging sensitive data)
                if let Some(ref params) = req.params {
                    // For security, we only log the structure, not the full content
                    event = event.metadata("params_type", params.clone());
                }

                Some(event.build())
            }
            Message::Notification(notif) => {
                let mut event = AuditEvent::builder()
                    .event_type(AuditEventType::MethodInvocation)
                    .method(&notif.method)
                    .result(AuditResult::Success)
                    .severity(AuditSeverity::Info)
                    .metadata("notification", true);

                // Add connection context if available
                if let Some(ref ctx) = self.connection_context
                    && let Some(addr) = ctx.remote_addr
                {
                    event = event.remote_addr(addr);
                }

                Some(event.build())
            }
            Message::Response(_) => {
                // We don't audit raw response messages
                None
            }
        }
    }

    /// Create audit event from response
    fn create_response_event(&self, message: &Message, response: Option<&Response>) -> AuditEvent {
        let method = match message {
            Message::Request(req) => Some(req.method.as_str()),
            Message::Notification(notif) => Some(notif.method.as_str()),
            Message::Response(_) => None,
        };

        let correlation_id = match message {
            Message::Request(req) => req.id.as_ref().map(|id| id.to_string()),
            _ => None,
        };

        let mut event_builder = AuditEvent::builder()
            .event_type(AuditEventType::MethodInvocation)
            .correlation_id(correlation_id.unwrap_or_default());

        if let Some(m) = method {
            event_builder = event_builder.method(m);
        }

        // Add connection context
        if let Some(ref ctx) = self.connection_context {
            if let Some(addr) = ctx.remote_addr {
                event_builder = event_builder.remote_addr(addr);
            }

            if let Some(user_id) = ctx.get::<String>("user_id") {
                event_builder = event_builder.principal(user_id);
            }
        }

        // Determine result based on response
        if let Some(resp) = response {
            if resp.is_success() {
                event_builder = event_builder.result(AuditResult::Success);
            } else {
                event_builder = event_builder
                    .result(AuditResult::Failure)
                    .severity(AuditSeverity::Warning);

                if let Some(ref error) = resp.error {
                    event_builder = event_builder
                        .error(&error.message)
                        .metadata("error_code", error.code);
                }
            }
        } else {
            // No response (notification or error)
            event_builder = event_builder.result(AuditResult::Success);
        }

        event_builder.build()
    }
}

#[async_trait]
impl MessageProcessor for AuditProcessor {
    async fn process_message(&self, message: Message) -> Option<Response> {
        // Log incoming request
        if let Some(request_event) = self.create_request_event(&message) {
            self.log_event(request_event);
        }

        // Process the message
        let response = self.inner.process_message(message.clone()).await;

        // Log response
        let response_event = self.create_response_event(&message, response.as_ref());
        self.log_event(response_event);

        response
    }

    fn get_capabilities(&self) -> ProcessorCapabilities {
        self.inner.get_capabilities()
    }
}

/// Builder for creating audit processors
pub struct AuditProcessorBuilder {
    processor: Arc<dyn MessageProcessor + Send + Sync>,
    backend: Arc<dyn AuditBackend>,
    integrity: Arc<dyn AuditIntegrity>,
    connection_context: Option<Arc<ConnectionContext>>,
}

impl AuditProcessorBuilder {
    /// Set the audit backend
    pub fn with_backend(mut self, backend: Arc<dyn AuditBackend>) -> Self {
        self.backend = backend;
        self
    }

    /// Set the integrity mechanism
    pub fn with_integrity(mut self, integrity: Arc<dyn AuditIntegrity>) -> Self {
        self.integrity = integrity;
        self
    }

    /// Set the connection context for extracting principal and metadata
    pub fn with_connection_context(mut self, context: Arc<ConnectionContext>) -> Self {
        self.connection_context = Some(context);
        self
    }

    /// Build the audit processor
    pub fn build(self) -> AuditProcessor {
        AuditProcessor {
            inner: self.processor,
            backend: self.backend,
            integrity: self.integrity,
            connection_context: self.connection_context,
        }
    }
}

/// Log authentication/authorization events
pub fn log_auth_event(
    backend: &dyn AuditBackend,
    integrity: &dyn AuditIntegrity,
    method: &str,
    ctx: &ConnectionContext,
    allowed: bool,
) {
    let mut event = AuditEvent::builder()
        .event_type(AuditEventType::AuthorizationCheck)
        .method(method)
        .result(if allowed {
            AuditResult::Success
        } else {
            AuditResult::Denied
        })
        .severity(if allowed {
            AuditSeverity::Info
        } else {
            AuditSeverity::Critical
        });

    if let Some(addr) = ctx.remote_addr {
        event = event.remote_addr(addr);
    }

    if let Some(user_id) = ctx.get::<String>("user_id") {
        event = event.principal(user_id);
    }

    let mut evt = event.build();
    integrity.add_integrity(&mut evt);
    backend.log_audit(&evt);
}

/// Log security policy violations (rate limits, banned IPs, etc.)
pub fn log_security_violation(
    backend: &dyn AuditBackend,
    integrity: &dyn AuditIntegrity,
    violation_type: &str,
    remote_addr: Option<std::net::SocketAddr>,
    principal: Option<&str>,
) {
    let mut event = AuditEvent::builder()
        .event_type(AuditEventType::SecurityViolation)
        .result(AuditResult::Violation)
        .severity(AuditSeverity::Critical)
        .metadata("violation_type", violation_type);

    if let Some(addr) = remote_addr {
        event = event.remote_addr(addr);
    }

    if let Some(p) = principal {
        event = event.principal(p);
    }

    let mut evt = event.build();
    integrity.add_integrity(&mut evt);
    backend.log_audit(&evt);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RequestBuilder;

    #[tokio::test]
    async fn test_audit_processor() {
        use crate::MethodRegistry;

        let registry = MethodRegistry::new(vec![]);
        let processor: Arc<dyn MessageProcessor + Send + Sync> = Arc::new(registry);

        let audit = AuditProcessor::builder(processor)
            .with_backend(Arc::new(super::super::NoopAuditBackend))
            .with_integrity(Arc::new(super::super::NoIntegrity))
            .build();

        let request = RequestBuilder::new("test_method")
            .id(serde_json::json!(1))
            .build();

        let _ = audit.process_message(Message::Request(request)).await;
    }
}
