//! Integrity verification mechanisms using sequence numbers, checksums, or combined checks.

use super::AuditEvent;
use std::sync::atomic::{AtomicU64, Ordering};

/// Audit integrity verification trait
pub trait AuditIntegrity: Send + Sync {
    /// Add integrity metadata to event
    fn add_integrity(&self, event: &mut AuditEvent);

    /// Verify event integrity (default: always passes)
    fn verify(&self, event: &AuditEvent) -> bool {
        let _ = event;
        true // Default: always pass
    }
}

/// No integrity checking
#[derive(Debug, Clone, Copy, Default)]
pub struct NoIntegrity;

impl AuditIntegrity for NoIntegrity {
    fn add_integrity(&self, _event: &mut AuditEvent) {
        // Intentionally no-op
    }
}

/// Adds monotonically increasing sequence numbers to detect gaps or duplicates
#[derive(Debug)]
pub struct SequenceIntegrity {
    sequence: AtomicU64,
}

impl SequenceIntegrity {
    /// Create a new sequence integrity checker starting at sequence 0
    pub fn new() -> Self {
        Self {
            sequence: AtomicU64::new(0),
        }
    }

    /// Create a new sequence integrity checker starting at a specific sequence number
    pub fn with_start(start: u64) -> Self {
        Self {
            sequence: AtomicU64::new(start),
        }
    }

    /// Get the current sequence number without incrementing
    pub fn current(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    /// Reset the sequence number
    pub fn reset(&self, value: u64) {
        self.sequence.store(value, Ordering::SeqCst);
    }
}

impl Default for SequenceIntegrity {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditIntegrity for SequenceIntegrity {
    fn add_integrity(&self, event: &mut AuditEvent) {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        event.add_metadata("sequence", seq);
    }

    fn verify(&self, event: &AuditEvent) -> bool {
        // Basic verification: ensure sequence number exists
        event.metadata.contains_key("sequence")
    }
}

/// Adds checksum of event fields to detect tampering
#[derive(Debug, Clone, Copy, Default)]
pub struct ChecksumIntegrity;

impl ChecksumIntegrity {
    /// Create a new checksum integrity checker
    pub fn new() -> Self {
        Self
    }

    /// Calculate checksum for an event
    fn calculate_checksum(event: &AuditEvent) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::UNIX_EPOCH;

        let mut hasher = DefaultHasher::new();

        // Hash event type
        format!("{:?}", event.event_type).hash(&mut hasher);

        // Hash correlation ID
        if let Some(ref id) = event.correlation_id {
            id.hash(&mut hasher);
        }

        // Hash principal
        if let Some(ref principal) = event.principal {
            principal.hash(&mut hasher);
        }

        // Hash method
        if let Some(ref method) = event.method {
            method.hash(&mut hasher);
        }

        // Hash result
        format!("{:?}", event.result).hash(&mut hasher);

        // Hash timestamp
        if let Ok(duration) = event.timestamp.duration_since(UNIX_EPOCH) {
            duration.as_nanos().hash(&mut hasher);
        }

        hasher.finish()
    }
}

impl AuditIntegrity for ChecksumIntegrity {
    fn add_integrity(&self, event: &mut AuditEvent) {
        let checksum = Self::calculate_checksum(event);
        event.add_metadata("checksum", checksum);
    }

    fn verify(&self, event: &AuditEvent) -> bool {
        // Extract stored checksum
        let stored_checksum = match event.metadata.get("checksum") {
            Some(serde_json::Value::Number(n)) => n.as_u64(),
            _ => return false,
        };

        if let Some(stored) = stored_checksum {
            // Create a temporary event without the checksum for verification
            let mut temp_event = event.clone();
            temp_event.metadata.remove("checksum");

            let calculated = Self::calculate_checksum(&temp_event);
            stored == calculated
        } else {
            false
        }
    }
}

/// Combines multiple integrity mechanisms
pub struct CombinedIntegrity {
    mechanisms: Vec<Box<dyn AuditIntegrity>>,
}

impl CombinedIntegrity {
    /// Create a new combined integrity checker
    pub fn new(mechanisms: Vec<Box<dyn AuditIntegrity>>) -> Self {
        Self { mechanisms }
    }

    /// Create a new combined integrity checker from Arc-wrapped mechanisms
    pub fn from_arcs(mechanisms: Vec<std::sync::Arc<dyn AuditIntegrity>>) -> Self {
        Self {
            mechanisms: mechanisms
                .into_iter()
                .map(|m| Box::new(ArcIntegrityWrapper(m)) as Box<dyn AuditIntegrity>)
                .collect(),
        }
    }
}

impl AuditIntegrity for CombinedIntegrity {
    fn add_integrity(&self, event: &mut AuditEvent) {
        for mechanism in &self.mechanisms {
            mechanism.add_integrity(event);
        }
    }

    fn verify(&self, event: &AuditEvent) -> bool {
        self.mechanisms.iter().all(|m| m.verify(event))
    }
}

/// Wrapper to make Arc<dyn AuditIntegrity> work with Box<dyn AuditIntegrity>
#[derive(Clone)]
struct ArcIntegrityWrapper(std::sync::Arc<dyn AuditIntegrity>);

impl AuditIntegrity for ArcIntegrityWrapper {
    fn add_integrity(&self, event: &mut AuditEvent) {
        self.0.add_integrity(event);
    }

    fn verify(&self, event: &AuditEvent) -> bool {
        self.0.verify(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit_logging::{AuditEventType, AuditResult};

    #[test]
    fn test_no_integrity() {
        let integrity = NoIntegrity;
        let mut event = AuditEvent::builder()
            .event_type(AuditEventType::MethodInvocation)
            .result(AuditResult::Success)
            .build();

        integrity.add_integrity(&mut event);
        assert!(event.metadata.is_empty());
        assert!(integrity.verify(&event));
    }

    #[test]
    fn test_sequence_integrity() {
        let integrity = SequenceIntegrity::new();

        let mut event1 = AuditEvent::builder()
            .event_type(AuditEventType::MethodInvocation)
            .result(AuditResult::Success)
            .build();
        integrity.add_integrity(&mut event1);

        let mut event2 = AuditEvent::builder()
            .event_type(AuditEventType::MethodInvocation)
            .result(AuditResult::Success)
            .build();
        integrity.add_integrity(&mut event2);

        // Check sequence numbers
        let seq1 = event1.metadata.get("sequence").unwrap().as_u64().unwrap();
        let seq2 = event2.metadata.get("sequence").unwrap().as_u64().unwrap();

        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);
        assert!(integrity.verify(&event1));
        assert!(integrity.verify(&event2));
    }

    #[test]
    fn test_checksum_integrity() {
        let integrity = ChecksumIntegrity::new();

        let mut event = AuditEvent::builder()
            .event_type(AuditEventType::AuthenticationAttempt)
            .principal("user@example.com")
            .method("login")
            .result(AuditResult::Success)
            .build();

        integrity.add_integrity(&mut event);
        assert!(event.metadata.contains_key("checksum"));
        assert!(integrity.verify(&event));

        // Tamper with the event
        event.principal = Some("attacker@example.com".to_string());
        assert!(!integrity.verify(&event));
    }

    #[test]
    fn test_combined_integrity() {
        let combined = CombinedIntegrity::new(vec![
            Box::new(SequenceIntegrity::new()),
            Box::new(ChecksumIntegrity::new()),
        ]);

        let mut event = AuditEvent::builder()
            .event_type(AuditEventType::MethodInvocation)
            .result(AuditResult::Success)
            .build();

        combined.add_integrity(&mut event);
        assert!(event.metadata.contains_key("sequence"));
        assert!(event.metadata.contains_key("checksum"));
        assert!(combined.verify(&event));
    }
}
