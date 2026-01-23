//! Pluggable audit logging backends for writing events to various destinations.

use super::AuditEvent;
use std::io::Write;

/// Audit log backend trait. Synchronous writes ensure events persist before execution continues.
pub trait AuditBackend: Send + Sync {
    /// Write an audit event
    fn log_audit(&self, event: &AuditEvent);

    /// Flush buffered entries
    fn flush(&self) {
        // Default: no-op
    }
}

/// Writes audit events to stdout as JSON lines
#[derive(Debug, Clone, Copy, Default)]
pub struct StdoutAuditBackend;

impl AuditBackend for StdoutAuditBackend {
    fn log_audit(&self, event: &AuditEvent) {
        match serde_json::to_string(event) {
            Ok(json) => {
                println!("{}", json);
            }
            Err(e) => {
                eprintln!("[AUDIT ERROR] Failed to serialize audit event: {}", e);
            }
        }
    }

    fn flush(&self) {
        let _ = std::io::stdout().flush();
    }
}

/// Writes audit events to stderr as JSON lines
#[derive(Debug, Clone, Copy, Default)]
pub struct StderrAuditBackend;

impl AuditBackend for StderrAuditBackend {
    fn log_audit(&self, event: &AuditEvent) {
        match serde_json::to_string(event) {
            Ok(json) => {
                eprintln!("{}", json);
            }
            Err(e) => {
                eprintln!(
                    "[AUDIT ERROR] Failed to serialize audit event: {}",
                    e
                );
            }
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

/// Discards all audit events (testing only)
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopAuditBackend;

impl AuditBackend for NoopAuditBackend {
    fn log_audit(&self, _event: &AuditEvent) {
        // Intentionally discard
    }
}

/// Writes audit events to multiple backends simultaneously
pub struct MultiAuditBackend {
    backends: Vec<Box<dyn AuditBackend>>,
}

impl MultiAuditBackend {
    /// Create a new multi-backend logger
    pub fn new(backends: Vec<Box<dyn AuditBackend>>) -> Self {
        Self { backends }
    }

    /// Create a new multi-backend logger from Arc-wrapped backends
    pub fn from_arcs(backends: Vec<std::sync::Arc<dyn AuditBackend>>) -> Self {
        Self {
            backends: backends
                .into_iter()
                .map(|b| Box::new(ArcBackendWrapper(b)) as Box<dyn AuditBackend>)
                .collect(),
        }
    }

    /// Add a backend
    pub fn add_backend(&mut self, backend: Box<dyn AuditBackend>) {
        self.backends.push(backend);
    }
}

impl AuditBackend for MultiAuditBackend {
    fn log_audit(&self, event: &AuditEvent) {
        for backend in &self.backends {
            backend.log_audit(event);
        }
    }

    fn flush(&self) {
        for backend in &self.backends {
            backend.flush();
        }
    }
}

/// Wrapper to make Arc<dyn AuditBackend> work with Box<dyn AuditBackend>
#[derive(Clone)]
struct ArcBackendWrapper(std::sync::Arc<dyn AuditBackend>);

impl AuditBackend for ArcBackendWrapper {
    fn log_audit(&self, event: &AuditEvent) {
        self.0.log_audit(event);
    }

    fn flush(&self) {
        self.0.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit_logging::{AuditEventType, AuditResult};

    #[test]
    fn test_noop_backend() {
        let backend = NoopAuditBackend;
        let event = AuditEvent::builder()
            .event_type(AuditEventType::MethodInvocation)
            .result(AuditResult::Success)
            .build();

        backend.log_audit(&event); // Should not panic
        backend.flush(); // Should not panic
    }

    #[test]
    fn test_multi_backend() {
        let multi = MultiAuditBackend::new(vec![
            Box::new(NoopAuditBackend),
            Box::new(NoopAuditBackend),
        ]);

        let event = AuditEvent::builder()
            .event_type(AuditEventType::AuthenticationAttempt)
            .result(AuditResult::Success)
            .build();

        multi.log_audit(&event);
        multi.flush();
    }
}
