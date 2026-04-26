//! OCSF (Open Cybersecurity Schema Framework) 1.4.0 support for audit logging.
//!
//! Provides conversion from [`AuditEvent`] to OCSF-compliant event structures and
//! ready-to-use backends that emit OCSF JSON for ingestion by SIEM platforms.
//!
//! ## Class mappings
//!
//! | `AuditEventType`         | OCSF class                       | class_uid |
//! |--------------------------|----------------------------------|-----------|
//! | `MethodInvocation`       | API Activity                     | 6003      |
//! | `AuthenticationAttempt`  | Authentication                   | 3002      |
//! | `AuthorizationCheck`     | Authorize Session                | 3003      |
//! | `SecurityViolation`      | Security Finding                 | 2001      |
//! | `ConnectionEstablished`  | Network Activity                 | 4001      |
//! | `ConnectionClosed`       | Network Activity                 | 4001      |
//! | `ErrorOccurred`          | API Activity                     | 6003      |
//! | `ConfigurationChange`    | Entity Management                | 3004      |
//! | `AdminAction`            | Account Change                   | 3001      |

use super::{AuditBackend, AuditEvent, AuditEventType, AuditResult, AuditSeverity};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

/// OCSF schema version implemented by this module.
pub const OCSF_SCHEMA_VERSION: &str = "1.4.0";

// ─── Sub-objects ────────────────────────────────────────────────────────────

/// Product that generated the event (required inside [`OcsfMetadata`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcsfProduct {
    /// Product display name.
    pub name: String,
    /// Vendor / organisation name.
    pub vendor_name: String,
    /// Product version string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// URL to the product homepage or documentation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_string: Option<String>,
}

impl OcsfProduct {
    /// Convenience constructor.
    #[must_use]
    pub fn new(name: impl Into<String>, vendor_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            vendor_name: vendor_name.into(),
            version: None,
            url_string: None,
        }
    }

    /// Set the product version.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set the product URL.
    #[must_use]
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url_string = Some(url.into());
        self
    }
}

impl Default for OcsfProduct {
    fn default() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME").to_string(),
            vendor_name: "ash-rpc".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            url_string: Some("https://github.com/ashforge-rs/ash-rpc".to_string()),
        }
    }
}

/// OCSF `metadata` object — required in every event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcsfMetadata {
    /// OCSF schema version (e.g. `"1.4.0"`).
    pub version: String,
    /// Originating product.
    pub product: OcsfProduct,
    /// Optional unique identifier for this log record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    /// Epoch-millisecond timestamp of when the event was logged (may differ from `time`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logged_time: Option<i64>,
    /// Free-form log level label (e.g. `"INFO"`, `"WARN"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_level: Option<String>,
    /// Applied OCSF profiles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profiles: Option<Vec<String>>,
    /// Arbitrary labels attached by the logging pipeline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
}

/// OCSF `user` object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcsfUser {
    /// Unique identifier of the user (UID, UUID, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    /// Display name of the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// User type (e.g. `"User"`, `"Admin"`, `"Service"`).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub user_type: Option<String>,
}

/// OCSF `session` object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcsfSession {
    /// Session identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    /// Epoch-millisecond creation time of the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_time: Option<i64>,
}

/// OCSF `actor` object — describes the entity that triggered the event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcsfActor {
    /// User associated with the action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<OcsfUser>,
    /// Session in which the action occurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<OcsfSession>,
    /// Application / service name acting on behalf of the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
}

/// OCSF network endpoint (used for `src_endpoint` / `dst_endpoint`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcsfNetworkEndpoint {
    /// IP address of the endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    /// TCP/UDP port.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Hostname of the endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
}

/// OCSF `api.service` object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcsfApiService {
    /// Service name.
    pub name: String,
}

/// OCSF `api.request` object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcsfApiRequest {
    /// Request identifier (e.g. JSON-RPC `id`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    /// Sanitised request payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Request flags / options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Vec<String>>,
}

/// OCSF `api.response` object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcsfApiResponse {
    /// HTTP-equivalent status code if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
    /// Human-readable response or error message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response flags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Vec<String>>,
}

/// OCSF `api` object — populated for API Activity events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcsfApi {
    /// The RPC method / operation name.
    pub operation: String,
    /// Protocol version (e.g. `"2.0"` for JSON-RPC 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Service that exposes the API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<OcsfApiService>,
    /// Inbound request details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<OcsfApiRequest>,
    /// Outbound response details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<OcsfApiResponse>,
}

// ─── Main event ─────────────────────────────────────────────────────────────

/// A fully-formed OCSF 1.4.0 event.
///
/// All required base-event fields are present. Optional fields use
/// `#[serde(skip_serializing_if = "Option::is_none")]` so the output stays
/// minimal.
///
/// Construct via [`to_ocsf`] or the [`OcsfStdoutBackend`] / [`OcsfStderrBackend`]
/// backends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcsfEvent {
    // ── Required classification ──────────────────────────────────────────────
    /// Numeric OCSF class identifier (e.g. `6003` for API Activity).
    pub class_uid: i32,
    /// Human-readable class name.
    pub class_name: String,
    /// Numeric OCSF category identifier.
    pub category_uid: i32,
    /// Human-readable category name.
    pub category_name: String,
    /// Activity identifier within the class.
    pub activity_id: i32,
    /// Human-readable activity name.
    pub activity_name: String,

    // ── Required type UID ────────────────────────────────────────────────────
    /// Composite type identifier: `class_uid * 100 + activity_id`.
    pub type_uid: i64,
    /// Human-readable type name: `"<class_name>: <activity_name>"`.
    pub type_name: String,

    // ── Required timing ──────────────────────────────────────────────────────
    /// Event time as epoch milliseconds.
    pub time: i64,

    // ── Required severity ────────────────────────────────────────────────────
    /// Numeric OCSF severity identifier.
    pub severity_id: i32,
    /// Human-readable severity label.
    pub severity: String,

    // ── Required metadata ────────────────────────────────────────────────────
    /// Event metadata block.
    pub metadata: OcsfMetadata,

    // ── Optional status ──────────────────────────────────────────────────────
    /// Numeric OCSF status identifier (1 = Success, 2 = Failure, 0 = Unknown).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_id: Option<i32>,
    /// Human-readable status label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Additional detail about the status (e.g. error message).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_detail: Option<String>,

    // ── Optional actor / endpoint / API ─────────────────────────────────────
    /// Actor that initiated the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<OcsfActor>,
    /// Source network endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src_endpoint: Option<OcsfNetworkEndpoint>,
    /// API details (populated for API Activity events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<OcsfApi>,

    // ── Optional correlation / message ──────────────────────────────────────
    /// Correlation / trace identifier (maps from JSON-RPC `id`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_uid: Option<String>,
    /// Free-form human-readable message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    // ── Unmapped extra fields ────────────────────────────────────────────────
    /// Extra fields from the source event that do not have a direct OCSF mapping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unmapped: Option<serde_json::Value>,
}

// ─── Conversion helpers ──────────────────────────────────────────────────────

/// Returns `(class_uid, category_uid, class_name, category_name, activity_id, activity_name)`
/// for the given event type / result combination.
fn classify(event_type: AuditEventType, result: AuditResult) -> (i32, i32, &'static str, &'static str, i32, &'static str) {
    match event_type {
        AuditEventType::MethodInvocation | AuditEventType::ErrorOccurred => {
            // API Activity (6003), category Application Activity (6)
            // activity_id 99 = Other (RPC invoke doesn't map to CRUD)
            (6003, 6, "API Activity", "Application Activity", 99, "Other")
        }
        AuditEventType::AuthenticationAttempt => {
            // Authentication (3002), category IAM (3)
            // activity_id 1 = Logon
            (3002, 3, "Authentication", "Identity & Access Management", 1, "Logon")
        }
        AuditEventType::AuthorizationCheck => {
            // Authorize Session (3003), category IAM (3)
            // activity_id 1 = Authorize
            (3003, 3, "Authorize Session", "Identity & Access Management", 1, "Authorize")
        }
        AuditEventType::SecurityViolation => {
            // Security Finding (2001), category Findings (2)
            // activity_id 1 = Create (new finding raised)
            let _ = result;
            (2001, 2, "Security Finding", "Findings", 1, "Create")
        }
        AuditEventType::ConnectionEstablished => {
            // Network Activity (4001), category Network Activity (4)
            // activity_id 1 = Open
            (4001, 4, "Network Activity", "Network Activity", 1, "Open")
        }
        AuditEventType::ConnectionClosed => {
            // Network Activity (4001), category Network Activity (4)
            // activity_id 4 = Close
            (4001, 4, "Network Activity", "Network Activity", 4, "Close")
        }
        AuditEventType::ConfigurationChange => {
            // Entity Management (3004), category IAM (3)
            // activity_id 2 = Update
            (3004, 3, "Entity Management", "Identity & Access Management", 2, "Update")
        }
        AuditEventType::AdminAction => {
            // Account Change (3001), category IAM (3)
            // activity_id 99 = Other
            (3001, 3, "Account Change", "Identity & Access Management", 99, "Other")
        }
    }
}

/// Map an [`AuditSeverity`] to OCSF `(severity_id, severity_label)`.
fn map_severity(severity: AuditSeverity) -> (i32, &'static str) {
    match severity {
        AuditSeverity::Info => (1, "Informational"),
        AuditSeverity::Warning => (3, "Medium"),
        AuditSeverity::Critical => (5, "Critical"),
    }
}

/// Map an [`AuditResult`] to OCSF `(status_id, status_label)`.
fn map_status(result: AuditResult) -> (i32, &'static str) {
    match result {
        AuditResult::Success => (1, "Success"),
        AuditResult::Failure | AuditResult::Denied | AuditResult::Violation => (2, "Failure"),
    }
}

/// Convert a [`SystemTime`] to epoch milliseconds (`i64`).
fn to_epoch_ms(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| {
            #[allow(clippy::as_conversions)]
            (d.as_millis() as i64)
        })
        .unwrap_or(0)
}

// ─── Public conversion ───────────────────────────────────────────────────────

/// Convert an [`AuditEvent`] to an [`OcsfEvent`] using the supplied product info.
///
/// # Example
/// ```rust,ignore
/// use ash_rpc::audit_logging::ocsf::{to_ocsf, OcsfProduct};
///
/// let ocsf = to_ocsf(&audit_event, &OcsfProduct::default());
/// let json = serde_json::to_string(&ocsf)?;
/// ```
#[must_use]
pub fn to_ocsf(event: &AuditEvent, product: &OcsfProduct) -> OcsfEvent {
    let (class_uid, category_uid, class_name, category_name, activity_id, activity_name) =
        classify(event.event_type, event.result);

    let type_uid = i64::from(class_uid) * 100 + i64::from(activity_id);
    let type_name = format!("{class_name}: {activity_name}");

    let (severity_id, severity_label) = map_severity(event.severity);
    let (status_id, status_label) = map_status(event.result);

    // ── metadata ────────────────────────────────────────────────────────────
    let logged_time = to_epoch_ms(SystemTime::now());
    let log_level = Some(severity_label.to_string());

    let metadata = OcsfMetadata {
        version: OCSF_SCHEMA_VERSION.to_string(),
        product: product.clone(),
        uid: None,
        logged_time: Some(logged_time),
        log_level,
        profiles: None,
        labels: None,
    };

    // ── actor ───────────────────────────────────────────────────────────────
    let actor = event.principal.as_deref().map(|p| OcsfActor {
        user: Some(OcsfUser {
            uid: Some(p.to_string()),
            name: Some(p.to_string()),
            user_type: None,
        }),
        session: None,
        app_name: None,
    });

    // ── source endpoint ─────────────────────────────────────────────────────
    let src_endpoint = event.remote_addr.map(|addr| OcsfNetworkEndpoint {
        ip: Some(addr.ip().to_string()),
        port: Some(addr.port()),
        hostname: None,
    });

    // ── api object (only for API Activity events) ────────────────────────────
    let api = if class_uid == 6003 {
        event.method.as_deref().map(|m| {
            let request = event.correlation_id.as_deref().map(|cid| OcsfApiRequest {
                uid: Some(cid.to_string()),
                data: event.params.clone(),
                flags: None,
            });

            let response = event.error.as_deref().map(|err| OcsfApiResponse {
                code: None,
                message: Some(err.to_string()),
                flags: None,
            });

            OcsfApi {
                operation: m.to_string(),
                version: Some("2.0".to_string()),
                service: None,
                request,
                response,
            }
        })
    } else {
        None
    };

    // ── unmapped extras ──────────────────────────────────────────────────────
    // Preserve any metadata keys that have no direct OCSF field so no data
    // is silently dropped.
    let unmapped = if event.metadata.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(
            event
                .metadata
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        ))
    };

    // For non-API events the method goes into the message field.
    let message = if class_uid != 6003 {
        event.method.as_deref().map(|m| {
            format!(
                "{event_type:?}: {m}",
                event_type = event.event_type,
            )
        })
    } else {
        event.error.clone()
    };

    OcsfEvent {
        class_uid,
        class_name: class_name.to_string(),
        category_uid,
        category_name: category_name.to_string(),
        activity_id,
        activity_name: activity_name.to_string(),
        type_uid,
        type_name,
        time: to_epoch_ms(event.timestamp),
        severity_id,
        severity: severity_label.to_string(),
        metadata,
        status_id: Some(status_id),
        status: Some(status_label.to_string()),
        status_detail: event.error.clone(),
        actor,
        src_endpoint,
        api,
        correlation_uid: event.correlation_id.clone(),
        message,
        unmapped,
    }
}

// ─── Backends ────────────────────────────────────────────────────────────────

/// Writes OCSF JSON events to **stdout**, one per line.
///
/// # Example
/// ```rust,ignore
/// use ash_rpc::audit_logging::ocsf::{OcsfStdoutBackend, OcsfProduct};
///
/// let backend = OcsfStdoutBackend::new(OcsfProduct::default());
/// ```
pub struct OcsfStdoutBackend {
    product: OcsfProduct,
}

impl OcsfStdoutBackend {
    /// Create a new backend with the given product metadata.
    #[must_use]
    pub fn new(product: OcsfProduct) -> Self {
        Self { product }
    }
}

impl Default for OcsfStdoutBackend {
    fn default() -> Self {
        Self::new(OcsfProduct::default())
    }
}

impl AuditBackend for OcsfStdoutBackend {
    fn log_audit(&self, event: &AuditEvent) {
        let ocsf = to_ocsf(event, &self.product);
        match serde_json::to_string(&ocsf) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("[AUDIT/OCSF ERROR] serialization failed: {e}"),
        }
    }

    fn flush(&self) {
        drop(std::io::stdout().flush());
    }
}

/// Writes OCSF JSON events to **stderr**, one per line.
///
/// # Example
/// ```rust,ignore
/// use ash_rpc::audit_logging::ocsf::{OcsfStderrBackend, OcsfProduct};
///
/// let backend = OcsfStderrBackend::new(OcsfProduct::default());
/// ```
pub struct OcsfStderrBackend {
    product: OcsfProduct,
}

impl OcsfStderrBackend {
    /// Create a new backend with the given product metadata.
    #[must_use]
    pub fn new(product: OcsfProduct) -> Self {
        Self { product }
    }
}

impl Default for OcsfStderrBackend {
    fn default() -> Self {
        Self::new(OcsfProduct::default())
    }
}

impl AuditBackend for OcsfStderrBackend {
    fn log_audit(&self, event: &AuditEvent) {
        let ocsf = to_ocsf(event, &self.product);
        match serde_json::to_string(&ocsf) {
            Ok(json) => eprintln!("{json}"),
            Err(e) => eprintln!("[AUDIT/OCSF ERROR] serialization failed: {e}"),
        }
    }

    fn flush(&self) {
        drop(std::io::stderr().flush());
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit_logging::{AuditEventType, AuditResult};

    fn make_event(event_type: AuditEventType, result: AuditResult) -> AuditEvent {
        AuditEvent::builder()
            .event_type(event_type)
            .result(result)
            .build()
    }

    #[test]
    fn method_invocation_maps_to_api_activity() {
        let event = make_event(AuditEventType::MethodInvocation, AuditResult::Success);
        let ocsf = to_ocsf(&event, &OcsfProduct::default());
        assert_eq!(ocsf.class_uid, 6003);
        assert_eq!(ocsf.category_uid, 6);
        assert_eq!(ocsf.type_uid, 600_399); // 6003 * 100 + 99
        assert_eq!(ocsf.status_id, Some(1));
    }

    #[test]
    fn authentication_attempt_maps_to_authentication() {
        let event = make_event(AuditEventType::AuthenticationAttempt, AuditResult::Failure);
        let ocsf = to_ocsf(&event, &OcsfProduct::default());
        assert_eq!(ocsf.class_uid, 3002);
        assert_eq!(ocsf.category_uid, 3);
        assert_eq!(ocsf.type_uid, 300_201); // 3002 * 100 + 1
        assert_eq!(ocsf.status_id, Some(2));
    }

    #[test]
    fn authorization_check_maps_to_authorize_session() {
        let event = make_event(AuditEventType::AuthorizationCheck, AuditResult::Denied);
        let ocsf = to_ocsf(&event, &OcsfProduct::default());
        assert_eq!(ocsf.class_uid, 3003);
        assert_eq!(ocsf.type_uid, 300_301); // 3003 * 100 + 1
        assert_eq!(ocsf.status_id, Some(2));
    }

    #[test]
    fn security_violation_maps_to_security_finding() {
        let event = make_event(AuditEventType::SecurityViolation, AuditResult::Violation);
        let ocsf = to_ocsf(&event, &OcsfProduct::default());
        assert_eq!(ocsf.class_uid, 2001);
        assert_eq!(ocsf.category_uid, 2);
    }

    #[test]
    fn connection_established_maps_to_network_activity_open() {
        let event = make_event(AuditEventType::ConnectionEstablished, AuditResult::Success);
        let ocsf = to_ocsf(&event, &OcsfProduct::default());
        assert_eq!(ocsf.class_uid, 4001);
        assert_eq!(ocsf.activity_id, 1);
    }

    #[test]
    fn connection_closed_maps_to_network_activity_close() {
        let event = make_event(AuditEventType::ConnectionClosed, AuditResult::Success);
        let ocsf = to_ocsf(&event, &OcsfProduct::default());
        assert_eq!(ocsf.class_uid, 4001);
        assert_eq!(ocsf.activity_id, 4);
    }

    #[test]
    fn ocsf_event_is_valid_json() {
        let event = AuditEvent::builder()
            .event_type(AuditEventType::MethodInvocation)
            .method("rpc.ping")
            .principal("user:alice")
            .result(AuditResult::Success)
            .build();
        let ocsf = to_ocsf(&event, &OcsfProduct::default());
        let json = serde_json::to_string(&ocsf).expect("must serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("must be valid JSON");
        assert_eq!(parsed["class_uid"], 6003);
        assert_eq!(parsed["metadata"]["version"], OCSF_SCHEMA_VERSION);
        assert_eq!(parsed["actor"]["user"]["uid"], "user:alice");
        assert_eq!(parsed["api"]["operation"], "rpc.ping");
    }

    #[test]
    fn severity_mapping() {
        let info = make_event(AuditEventType::MethodInvocation, AuditResult::Success);
        assert_eq!(to_ocsf(&info, &OcsfProduct::default()).severity_id, 1);

        let warn = AuditEvent::builder()
            .event_type(AuditEventType::ErrorOccurred)
            .result(AuditResult::Failure)
            .build();
        assert_eq!(to_ocsf(&warn, &OcsfProduct::default()).severity_id, 3);

        let crit = AuditEvent::builder()
            .event_type(AuditEventType::SecurityViolation)
            .result(AuditResult::Violation)
            .build();
        assert_eq!(to_ocsf(&crit, &OcsfProduct::default()).severity_id, 5);
    }

    #[test]
    fn metadata_preserved_in_unmapped() {
        let event = AuditEvent::builder()
            .event_type(AuditEventType::MethodInvocation)
            .result(AuditResult::Success)
            .metadata("custom_key", "custom_value")
            .build();
        let ocsf = to_ocsf(&event, &OcsfProduct::default());
        let unmapped = ocsf.unmapped.expect("should have unmapped");
        assert_eq!(unmapped["custom_key"], "custom_value");
    }

    #[test]
    fn noop_backends_do_not_panic() {
        let event = make_event(AuditEventType::MethodInvocation, AuditResult::Success);
        // Just verify the backends can be constructed and called without panicking.
        // (stdout/stderr output is a side-effect we accept in tests.)
        let stdout = OcsfStdoutBackend::default();
        stdout.log_audit(&event);
        stdout.flush();

        let stderr = OcsfStderrBackend::default();
        stderr.log_audit(&event);
        stderr.flush();
    }
}
