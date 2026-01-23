//! Security audit logging for ash-rpc
//!
//! Provides structured, tamper-evident audit logging for security events including
//! authentication, authorization, method invocations, and policy violations.
//!
//! Features: append-only logs, integrity verification, pluggable backends, compliance-ready.

mod backends;
mod integrity;
mod processor;

pub use backends::*;
pub use integrity::*;
pub use processor::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::SystemTime;

/// A security audit event representing a significant action or decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Precise timestamp with nanosecond precision
    #[serde(with = "system_time_format")]
    pub timestamp: SystemTime,

    /// Type of audit event
    pub event_type: AuditEventType,

    /// Unique correlation ID spanning the request chain
    pub correlation_id: Option<String>,

    /// Remote address of the client
    pub remote_addr: Option<SocketAddr>,

    /// Principal identifier (user ID, API key, certificate DN, etc.)
    pub principal: Option<String>,

    /// Method or action being performed
    pub method: Option<String>,

    /// Result of the action
    pub result: AuditResult,

    /// Event severity level
    pub severity: AuditSeverity,

    /// Additional context and metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Request parameters (sanitized)
    pub params: Option<serde_json::Value>,

    /// Error message if result is Failure or Denied
    pub error: Option<String>,
}

/// Types of security-significant events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// Connection established
    ConnectionEstablished,

    /// Connection closed
    ConnectionClosed,

    /// Authentication attempt (login, certificate validation, etc.)
    AuthenticationAttempt,

    /// Authorization check (access control decision)
    AuthorizationCheck,

    /// RPC method invocation
    MethodInvocation,

    /// Error occurred during processing
    ErrorOccurred,

    /// Security policy violation (rate limit, size limit, banned IP, etc.)
    SecurityViolation,

    /// Configuration change (admin action)
    ConfigurationChange,

    /// System administrative action
    AdminAction,
}

/// Result of an audited action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditResult {
    /// Action succeeded
    Success,

    /// Action failed due to error
    Failure,

    /// Action denied by policy
    Denied,

    /// Action resulted in security violation
    Violation,
}

/// Severity level of audit event
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    /// Informational event
    Info,

    /// Warning event
    Warning,

    /// Critical security event
    Critical,
}

impl AuditEvent {
    /// Create a new audit event builder
    pub fn builder() -> AuditEventBuilder {
        AuditEventBuilder::default()
    }

    /// Add a metadata entry
    pub fn add_metadata<K: Into<String>, V: Into<serde_json::Value>>(&mut self, key: K, value: V) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Set the correlation ID from a request
    pub fn with_correlation_id(mut self, correlation_id: Option<String>) -> Self {
        self.correlation_id = correlation_id;
        self
    }

    /// Set the principal from connection context
    pub fn with_principal<S: Into<String>>(mut self, principal: S) -> Self {
        self.principal = Some(principal.into());
        self
    }

    /// Set the remote address
    pub fn with_remote_addr(mut self, addr: SocketAddr) -> Self {
        self.remote_addr = Some(addr);
        self
    }
}

/// Builder for creating audit events
#[derive(Debug, Default)]
pub struct AuditEventBuilder {
    event_type: Option<AuditEventType>,
    correlation_id: Option<String>,
    remote_addr: Option<SocketAddr>,
    principal: Option<String>,
    method: Option<String>,
    result: Option<AuditResult>,
    severity: Option<AuditSeverity>,
    metadata: HashMap<String, serde_json::Value>,
    params: Option<serde_json::Value>,
    error: Option<String>,
}

impl AuditEventBuilder {
    /// Set event type
    pub fn event_type(mut self, event_type: AuditEventType) -> Self {
        self.event_type = Some(event_type);
        self
    }

    /// Set correlation ID
    pub fn correlation_id<S: Into<String>>(mut self, id: S) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Set remote address
    pub fn remote_addr(mut self, addr: SocketAddr) -> Self {
        self.remote_addr = Some(addr);
        self
    }

    /// Set principal
    pub fn principal<S: Into<String>>(mut self, principal: S) -> Self {
        self.principal = Some(principal.into());
        self
    }

    /// Set method name
    pub fn method<S: Into<String>>(mut self, method: S) -> Self {
        self.method = Some(method.into());
        self
    }

    /// Set result
    pub fn result(mut self, result: AuditResult) -> Self {
        self.result = Some(result);
        self
    }

    /// Set severity
    pub fn severity(mut self, severity: AuditSeverity) -> Self {
        self.severity = Some(severity);
        self
    }

    /// Add metadata entry
    pub fn metadata<K: Into<String>, V: Into<serde_json::Value>>(
        mut self,
        key: K,
        value: V,
    ) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set sanitized parameters
    pub fn params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Set error message
    pub fn error<S: Into<String>>(mut self, error: S) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Build the audit event
    pub fn build(self) -> AuditEvent {
        let event_type = self.event_type.expect("event_type is required");
        let result = self.result.expect("result is required");

        // Determine default severity based on result
        let severity = self.severity.unwrap_or(match result {
            AuditResult::Success => AuditSeverity::Info,
            AuditResult::Failure => AuditSeverity::Warning,
            AuditResult::Denied | AuditResult::Violation => AuditSeverity::Critical,
        });

        AuditEvent {
            timestamp: SystemTime::now(),
            event_type,
            correlation_id: self.correlation_id,
            remote_addr: self.remote_addr,
            principal: self.principal,
            method: self.method,
            result,
            severity,
            metadata: self.metadata,
            params: self.params,
            error: self.error,
        }
    }
}

/// Custom serialization for SystemTime to include nanosecond precision
mod system_time_format {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        let nanos = duration.as_secs() * 1_000_000_000 + u64::from(duration.subsec_nanos());
        serializer.serialize_u64(nanos)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let nanos = u64::deserialize(deserializer)?;
        let secs = nanos / 1_000_000_000;
        let subsec_nanos = (nanos % 1_000_000_000) as u32;
        Ok(UNIX_EPOCH + std::time::Duration::new(secs, subsec_nanos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_builder() {
        let event = AuditEvent::builder()
            .event_type(AuditEventType::AuthenticationAttempt)
            .principal("user@example.com")
            .method("login")
            .result(AuditResult::Success)
            .build();

        assert_eq!(event.event_type, AuditEventType::AuthenticationAttempt);
        assert_eq!(event.principal, Some("user@example.com".to_string()));
        assert_eq!(event.result, AuditResult::Success);
        assert_eq!(event.severity, AuditSeverity::Info);
    }

    #[test]
    fn test_audit_event_serialization() {
        let event = AuditEvent::builder()
            .event_type(AuditEventType::MethodInvocation)
            .principal("test_user")
            .method("get_balance")
            .result(AuditResult::Success)
            .build();

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("method_invocation"));
        assert!(json.contains("test_user"));
        assert!(json.contains("success"));
    }

    #[test]
    fn test_severity_defaults() {
        let success = AuditEvent::builder()
            .event_type(AuditEventType::MethodInvocation)
            .result(AuditResult::Success)
            .build();
        assert_eq!(success.severity, AuditSeverity::Info);

        let denied = AuditEvent::builder()
            .event_type(AuditEventType::AuthorizationCheck)
            .result(AuditResult::Denied)
            .build();
        assert_eq!(denied.severity, AuditSeverity::Critical);
    }
}
