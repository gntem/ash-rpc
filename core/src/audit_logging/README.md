# Security Audit Logging

A comprehensive, tamper-evident audit logging system for ash-rpc that automatically captures all security-significant events.

## Overview

The audit logging module provides:

- **Immutable Log Stream**: Append-only audit trail of all security-relevant operations
- **Structured Events**: JSON-serialized events with nanosecond-precision timestamps
- **Correlation IDs**: Track requests across their entire lifecycle
- **Principal Tracking**: Capture authenticated user IDs, API keys, or client certificates
- **Integrity Verification**: Sequence numbers and checksums to detect tampering
- **Compliance Ready**: Designed for GDPR, SOC 2, HIPAA audit requirements
- **Pluggable Backends**: Write to stdout, stderr, files, syslog, or custom destinations

## Core Concepts

### Security-First Design

Unlike general application logging, security audit logs:
- **Cannot be disabled** in production (use NoopAuditBackend only for testing)
- **Record immutable facts** about security decisions
- **Include full context** (who, what, when, from where)
- **Are tamper-evident** through sequence numbers and checksums
- **Prioritize correctness over performance** (synchronous writes)

### Event Types

The system automatically logs:

| Event Type | Description | Severity |
|------------|-------------|----------|
| `connection_established` | New client connection | Info |
| `connection_closed` | Connection terminated | Info |
| `authentication_attempt` | Login/credential validation | Info/Critical |
| `authorization_check` | Access control decision | Info/Critical |
| `method_invocation` | RPC method called | Info |
| `error_occurred` | Error during processing | Warning |
| `security_violation` | Policy violation (rate limit, size limit, banned IP) | Critical |
| `configuration_change` | Security config updated | Warning |
| `admin_action` | Administrative operation | Critical |

## Quick Start

### Basic Setup

```rust
use ash_rpc_core::audit_logging::*;
use ash_rpc_core::{MethodRegistry, MessageProcessor};
use std::sync::Arc;

// Create your RPC processor
let methods = MethodRegistry::new(vec![/* your methods */]);
let processor: Arc<dyn MessageProcessor + Send + Sync> = Arc::new(methods);

// Wrap with audit logging
let audited = AuditProcessor::builder(processor)
    .with_backend(Arc::new(StdoutAuditBackend))
    .with_integrity(Arc::new(SequenceIntegrity::new()))
    .build();

// Use audited processor instead of raw processor
// All operations are now automatically logged
```

### With Authentication Context

```rust
use ash_rpc_core::auth::ConnectionContext;

// Create connection context with user info
let mut ctx = ConnectionContext::with_addr("127.0.0.1:54321".parse().unwrap());
ctx.insert("user_id".to_string(), "alice@example.com".to_string());
let ctx = Arc::new(ctx);

// Provide context to audit processor
let audited = AuditProcessor::builder(processor)
    .with_backend(Arc::new(StdoutAuditBackend))
    .with_integrity(Arc::new(SequenceIntegrity::new()))
    .with_connection_context(ctx)
    .build();
```

### Custom Auth Policy with Audit Logging

```rust
use ash_rpc_core::auth::{AuthPolicy, ConnectionContext};

struct AuditingAuthPolicy {
    backend: Arc<dyn AuditBackend>,
    integrity: Arc<dyn AuditIntegrity>,
}

impl AuthPolicy for AuditingAuthPolicy {
    fn can_access(&self, method: &str, _params: Option<&serde_json::Value>, ctx: &ConnectionContext) -> bool {
        let allowed = /* your auth logic */;
        
        // Log the authorization check
        log_auth_event(&*self.backend, &*self.integrity, method, ctx, allowed);
        
        allowed
    }
}
```

## Backends

### StdoutAuditBackend (Default)

Writes JSON-formatted audit events to stdout. Suitable for containerized environments where stdout is captured by log aggregators (e.g., Docker, Kubernetes).

```rust
let backend = Arc::new(StdoutAuditBackend);
```

**Output format:**
```json
{
  "timestamp": 1769122805500837647,
  "event_type": "method_invocation",
  "correlation_id": "abc-123",
  "remote_addr": "192.168.1.100:54321",
  "principal": "alice@example.com",
  "method": "transfer_funds",
  "result": "success",
  "severity": "info",
  "metadata": {"sequence": 42},
  "params": null,
  "error": null
}
```

### StderrAuditBackend

Writes to stderr. Useful for separating critical security events from normal application logs.

```rust
let backend = Arc::new(StderrAuditBackend);
```

### NoopAuditBackend

Discards all events. **Only use for testing!** Never use in production.

```rust
let backend = Arc::new(NoopAuditBackend);
```

### MultiAuditBackend

Write to multiple backends simultaneously.

```rust
let multi = Arc::new(MultiAuditBackend::from_arcs(vec![
    Arc::new(StdoutAuditBackend),
    Arc::new(StderrAuditBackend),
]));
```

## Integrity Mechanisms

### SequenceIntegrity

Adds monotonically increasing sequence numbers to detect:
- Missing events (sequence gaps)
- Duplicate events (repeated sequence numbers)
- Out-of-order delivery

```rust
let integrity = Arc::new(SequenceIntegrity::new());

// Or start at specific sequence
let integrity = Arc::new(SequenceIntegrity::with_start(1000));
```

### ChecksumIntegrity

Adds checksums based on event content to detect tampering.

```rust
let integrity = Arc::new(ChecksumIntegrity::new());
```

### CombinedIntegrity

Apply multiple integrity mechanisms.

```rust
let combined = Arc::new(CombinedIntegrity::from_arcs(vec![
    Arc::new(SequenceIntegrity::new()),
    Arc::new(ChecksumIntegrity::new()),
]));
```

### NoIntegrity

No integrity checking. Only use when integrity is guaranteed externally (e.g., write-once storage).

```rust
let integrity = Arc::new(NoIntegrity);
```

## Security Events

### Logging Security Violations

```rust
use ash_rpc_core::audit_logging::log_security_violation;

// Rate limit exceeded
log_security_violation(
    &*backend,
    &*integrity,
    "rate_limit_exceeded",
    Some(client_addr),
    Some("user@example.com"),
);

// Banned IP connection
log_security_violation(
    &*backend,
    &*integrity,
    "banned_ip_connection",
    Some(banned_addr),
    None,
);

// Oversized request
log_security_violation(
    &*backend,
    &*integrity,
    "request_size_limit_exceeded",
    Some(client_addr),
    Some("api_key:abc123"),
);
```

### Custom Security Events

```rust
let mut event = AuditEvent::builder()
    .event_type(AuditEventType::SecurityViolation)
    .result(AuditResult::Violation)
    .severity(AuditSeverity::Critical)
    .principal("attacker@evil.com")
    .remote_addr("198.51.100.1:9999".parse().unwrap())
    .metadata("violation_type", "sql_injection_attempt")
    .metadata("attack_pattern", "' OR '1'='1")
    .error("SQL injection pattern detected")
    .build();

integrity.add_integrity(&mut event);
backend.log_audit(&event);
```

## Event Structure

Every audit event contains:

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | u64 | Nanoseconds since Unix epoch |
| `event_type` | Enum | Type of security event |
| `correlation_id` | String | Request/response correlation ID |
| `remote_addr` | SocketAddr | Client IP and port |
| `principal` | String | Authenticated user/principal |
| `method` | String | RPC method name |
| `result` | Enum | Success, Failure, Denied, or Violation |
| `severity` | Enum | Info, Warning, or Critical |
| `metadata` | Map | Additional context (sequence, checksum, etc.) |
| `params` | Value | Sanitized request parameters |
| `error` | String | Error message if applicable |

## Compliance Considerations

### GDPR

- Set retention policies on log storage (90-day minimum)
- Sanitize personal data in parameters
- Include data subject identifiers in `principal` field
- Log access to personal data

### SOC 2

- Enable integrity checking (sequence + checksum)
- Log all authentication/authorization events
- Maintain immutable audit trail
- Include correlation IDs for traceability

### HIPAA

- Log all access to protected health information (PHI)
- Include user identifiers and timestamps
- Maintain audit logs for 6+ years
- Protect log confidentiality (encrypt in transit/at rest)

## Performance Considerations

### Synchronous by Design

Audit logging is **intentionally synchronous** to ensure:
- Events are written before actions complete
- No log loss on crashes or shutdowns
- Accurate ordering of security events

This prioritizes **security over performance**. The performance impact is minimal for most use cases.

### When Performance Matters

If audit logging becomes a bottleneck:

1. **Use multi-backend** to separate critical events (stderr) from informational (stdout)
2. **Filter events at the source** - only log what's required for compliance
3. **Optimize backend** - use buffered file I/O or async network senders
4. **Consider external aggregation** - let log collectors handle async delivery

## Example Output

```json
{"timestamp":1769122805500837647,"event_type":"authorization_check","correlation_id":null,"remote_addr":"127.0.0.1:54321","principal":"alice@example.com","method":"admin_delete_user","result":"denied","severity":"critical","metadata":{"sequence":42},"params":null,"error":null}
```

## Best Practices

1. **Always enable in production** - Use `NoopAuditBackend` only for tests
2. **Include connection context** - Provide `ConnectionContext` to capture principals
3. **Use sequence integrity** - Minimum integrity mechanism for compliance
4. **Log auth denials** - All authorization failures should be logged
5. **Sanitize parameters** - Never log raw sensitive data
6. **Monitor sequence gaps** - Alert on missing sequence numbers
7. **Rotate log storage** - Configure retention based on compliance requirements
8. **Encrypt at rest** - Protect audit logs like production data
9. **Test integrity verification** - Regularly verify checksums and sequences
10. **Document audit policy** - Maintain written policy for compliance audits

## Running the Example

```bash
cargo run --example audit_logging_example --features audit-logging
```

This demonstrates:
- Basic audit logging setup
- Authentication integration
- Multi-backend configuration
- Security violation logging
- Integrity verification

## Testing

```bash
cargo test --features audit-logging audit_logging
```

## Feature Flag

Add to your `Cargo.toml`:

```toml
[dependencies]
ash-rpc-core = { version = "3.2.1", features = ["audit-logging"] }
```

## Future Enhancements

Potential extensions (not yet implemented):

- **File backend** with log rotation
- **Syslog backend** for centralized logging
- **Database backend** for structured storage
- **SIEM integration** (Splunk, ELK, Azure Sentinel)
- **Cryptographic signatures** for legal non-repudiation
- **Async buffering** with explicit flush guarantees
- **Query API** for forensic investigation
- **Alerting** on security event patterns
