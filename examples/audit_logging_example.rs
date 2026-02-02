//! Comprehensive audit logging example
//!
//! This example demonstrates:
//! - Setting up audit logging with different backends
//! - Integrating with authentication/authorization
//! - Capturing security events (auth failures, violations)
//! - Using sequence integrity checking
//! - Enriching events with connection context

use ash_rpc::audit_logging::*;
use ash_rpc::{
    MessageProcessor, MethodRegistry, RequestBuilder, Response, ResponseBuilder,
    auth::{AuthPolicy, ConnectionContext},
};
use std::net::SocketAddr;
use std::sync::Arc;

/// Custom auth policy that logs all authentication attempts
struct AuditingAuthPolicy {
    backend: Arc<dyn AuditBackend>,
    integrity: Arc<dyn AuditIntegrity>,
    allowed_methods: Vec<String>,
}

impl AuditingAuthPolicy {
    fn new(backend: Arc<dyn AuditBackend>, integrity: Arc<dyn AuditIntegrity>) -> Self {
        Self {
            backend,
            integrity,
            allowed_methods: vec![
                "get_public_data".to_string(),
                "echo".to_string(),
                "ping".to_string(),
            ],
        }
    }
}

impl AuthPolicy for AuditingAuthPolicy {
    fn can_access(
        &self,
        method: &str,
        _params: Option<&serde_json::Value>,
        ctx: &ConnectionContext,
    ) -> bool {
        let allowed = self.allowed_methods.contains(&method.to_string());

        // Log the authorization check
        log_auth_event(&*self.backend, &*self.integrity, method, ctx, allowed);

        allowed
    }

    fn unauthorized_error(&self, method: &str) -> Response {
        ResponseBuilder::new()
            .error(
                ash_rpc::ErrorBuilder::new(
                    ash_rpc::error_codes::INTERNAL_ERROR,
                    format!("Access denied to method: {}", method),
                )
                .build(),
            )
            .id(None)
            .build()
    }
}

#[tokio::main]
async fn main() {
    println!("=== Audit Logging Example ===\n");

    // Setup 1: Simple stdout audit logging with sequence integrity
    example_basic_audit().await;
    println!("\n{}\n", "=".repeat(50));

    // Setup 2: Audit logging with authentication
    example_audit_with_auth().await;
    println!("\n{}\n", "=".repeat(50));

    // Setup 3: Multi-backend with different integrity mechanisms
    example_multi_backend().await;
    println!("\n{}\n", "=".repeat(50));

    // Setup 4: Security violations
    example_security_violations().await;
}

/// Example 1: Basic audit logging setup
async fn example_basic_audit() {
    println!("Example 1: Basic Audit Logging with Stdout\n");

    // Create audit backend and integrity checker
    let backend = Arc::new(StdoutAuditBackend);
    let integrity = Arc::new(SequenceIntegrity::new());

    // Create a simple method registry
    let methods = MethodRegistry::new(vec![]);
    let processor: Arc<dyn MessageProcessor + Send + Sync> = Arc::new(methods);

    // Wrap with audit processor
    let audit_processor = AuditProcessor::builder(processor)
        .with_backend(backend)
        .with_integrity(integrity)
        .build();

    // Simulate processing a request
    let request = RequestBuilder::new("get_balance")
        .id(serde_json::json!(1))
        .params(serde_json::json!({"account_id": "12345"}))
        .build();

    println!("Processing request: get_balance");
    let _ = audit_processor
        .process_message(ash_rpc::Message::Request(request))
        .await;
    println!("Audit events logged to stdout (check above for JSON output)\n");
}

/// Example 2: Audit logging with authentication integration
async fn example_audit_with_auth() {
    println!("Example 2: Audit Logging with Authentication\n");

    // Create audit infrastructure
    let backend: Arc<dyn AuditBackend> = Arc::new(StdoutAuditBackend);
    let integrity: Arc<dyn AuditIntegrity> = Arc::new(SequenceIntegrity::new());

    // Create auth policy that uses audit logging
    let auth_policy = AuditingAuthPolicy::new(Arc::clone(&backend), Arc::clone(&integrity));

    // Create method registry with auth policy
    let methods = MethodRegistry::new(vec![]).with_auth(auth_policy);

    let processor: Arc<dyn MessageProcessor + Send + Sync> = Arc::new(methods);

    // Create connection context with user info
    let mut conn_ctx = ConnectionContext::with_addr("127.0.0.1:54321".parse().unwrap());
    conn_ctx.insert("user_id".to_string(), "alice@example.com".to_string());
    let conn_ctx = Arc::new(conn_ctx);

    // Wrap with audit processor including connection context
    let audit_processor = AuditProcessor::builder(processor)
        .with_backend(backend)
        .with_integrity(integrity)
        .with_connection_context(conn_ctx)
        .build();

    // Test allowed method
    println!("Test 1: Accessing allowed method (get_public_data)");
    let request = RequestBuilder::new("get_public_data")
        .id(serde_json::json!(1))
        .build();

    let _ = audit_processor
        .process_message(ash_rpc::Message::Request(request))
        .await;
    println!("Authorization check logged (allowed)\n");

    // Test denied method
    println!("Test 2: Accessing denied method (admin_delete_user)");
    let request = RequestBuilder::new("admin_delete_user")
        .id(serde_json::json!(2))
        .params(serde_json::json!({"user_id": "bob@example.com"}))
        .build();

    let _ = audit_processor
        .process_message(ash_rpc::Message::Request(request))
        .await;
    println!("Authorization denial logged (denied - critical severity)\n");
}

/// Example 3: Multiple backends and integrity mechanisms
async fn example_multi_backend() {
    println!("Example 3: Multi-Backend with Combined Integrity\n");

    // Create multiple backends
    let multi_backend = Arc::new(MultiAuditBackend::from_arcs(vec![
        Arc::new(StdoutAuditBackend),
        Arc::new(StderrAuditBackend), // Critical events also go to stderr
    ]));

    // Combine multiple integrity mechanisms
    let combined_integrity = Arc::new(CombinedIntegrity::from_arcs(vec![
        Arc::new(SequenceIntegrity::with_start(1000)), // Start at 1000
        Arc::new(ChecksumIntegrity::new()),            // Add checksum
    ]));

    // Create processor with multi-backend
    let methods = MethodRegistry::new(vec![]);
    let processor: Arc<dyn MessageProcessor + Send + Sync> = Arc::new(methods);

    let audit_processor = AuditProcessor::builder(processor)
        .with_backend(multi_backend)
        .with_integrity(combined_integrity)
        .build();

    // Process request
    println!("Processing request with both sequence and checksum integrity");
    let request = RequestBuilder::new("transfer_funds")
        .id(serde_json::json!(1))
        .params(serde_json::json!({
            "from": "account_123",
            "to": "account_456",
            "amount": 1000.50
        }))
        .build();

    let _ = audit_processor
        .process_message(ash_rpc::Message::Request(request))
        .await;
    println!("Events logged to both stdout and stderr with sequence+checksum\n");
}

/// Example 4: Logging security violations
async fn example_security_violations() {
    println!("Example 4: Security Violation Logging\n");

    let backend = Arc::new(StdoutAuditBackend);
    let integrity = Arc::new(SequenceIntegrity::new());

    // Simulate rate limit violation
    println!("Scenario 1: Rate limit exceeded");
    let addr: SocketAddr = "192.168.1.100:12345".parse().unwrap();
    log_security_violation(
        &*backend,
        &*integrity,
        "rate_limit_exceeded",
        Some(addr),
        Some("user@example.com"),
    );
    println!("Rate limit violation logged\n");

    // Simulate banned IP connection
    println!("Scenario 2: Connection from banned IP");
    let banned_addr: SocketAddr = "10.0.0.5:54321".parse().unwrap();
    log_security_violation(
        &*backend,
        &*integrity,
        "banned_ip_connection",
        Some(banned_addr),
        None,
    );
    println!("Banned IP violation logged\n");

    // Simulate oversized request
    println!("Scenario 3: Oversized request blocked");
    log_security_violation(
        &*backend,
        &*integrity,
        "request_size_limit_exceeded",
        Some("203.0.113.42:8080".parse().unwrap()),
        Some("api_key:abc123"),
    );
    println!("Request size violation logged\n");

    // Manually create a custom security event
    println!("Scenario 4: Custom security event");
    let mut event = AuditEvent::builder()
        .event_type(AuditEventType::SecurityViolation)
        .result(AuditResult::Violation)
        .severity(AuditSeverity::Critical)
        .method("suspicious_activity")
        .principal("attacker@evil.com")
        .remote_addr("198.51.100.1:9999".parse().unwrap())
        .metadata("violation_type", "sql_injection_attempt")
        .metadata("attack_pattern", "' OR '1'='1")
        .error("SQL injection pattern detected in parameters")
        .build();

    integrity.add_integrity(&mut event);
    backend.log_audit(&event);
    println!("Custom security event logged\n");
}

/// Helper to demonstrate event verification
#[allow(dead_code)]
fn demonstrate_integrity_verification() {
    println!("=== Integrity Verification Demo ===\n");

    let integrity = ChecksumIntegrity::new();

    // Create an event
    let mut event = AuditEvent::builder()
        .event_type(AuditEventType::MethodInvocation)
        .principal("user@example.com")
        .method("sensitive_operation")
        .result(AuditResult::Success)
        .build();

    // Add integrity
    integrity.add_integrity(&mut event);

    // Verify (should pass)
    println!("Original event verification: {}", integrity.verify(&event));

    // Tamper with event
    event.principal = Some("attacker@evil.com".to_string());

    // Verify (should fail)
    println!(
        "Tampered event verification: {} (should be false)",
        integrity.verify(&event)
    );
}
