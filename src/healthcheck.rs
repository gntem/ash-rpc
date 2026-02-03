//! Health check JSON-RPC method implementation
//!
//! Provides a standard health check method that can be used to monitor
//! service availability and health status.

use crate::*;
use serde::{Deserialize, Serialize};

/// Health check response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: u64,
    pub service: String,
    pub version: Option<String>,
}

/// JSON-RPC health check method implementation
///
/// Responds to "healthcheck" calls with service status information.
/// Parameters are ignored - this method always returns the current health status.
pub struct HealthcheckMethod {
    service_name: String,
    version: Option<String>,
}

impl HealthcheckMethod {
    /// Create a new healthcheck method with default service name
    pub fn new() -> Self {
        Self {
            service_name: "ash-rpc-service".to_string(),
            version: None,
        }
    }

    /// Create a healthcheck method with custom service name
    pub fn with_service_name(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            version: None,
        }
    }

    /// Set the service name (builder pattern)
    pub fn service_name(mut self, service_name: impl Into<String>) -> Self {
        self.service_name = service_name.into();
        self
    }

    /// Set the service version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
}

impl Default for HealthcheckMethod {
    fn default() -> Self {
        Self::new()
    }
}

#[crate::async_trait]
impl JsonRPCMethod for HealthcheckMethod {
    fn method_name(&self) -> &'static str {
        "healthcheck"
    }

    async fn call(&self, _params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        let health_status = HealthStatus {
            status: "healthy".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            service: self.service_name.clone(),
            version: self.version.clone(),
        };

        match serde_json::to_value(health_status) {
            Ok(status_json) => rpc_success!(status_json, id),
            Err(_) => rpc_error!(
                error_codes::INTERNAL_ERROR,
                "Failed to serialize health status",
                id
            ),
        }
    }
}

/// Convenience function to create a healthcheck method
pub fn healthcheck() -> HealthcheckMethod {
    HealthcheckMethod::new()
}

/// Convenience function to create a healthcheck method with service name
pub fn healthcheck_with_service(service_name: impl Into<String>) -> HealthcheckMethod {
    HealthcheckMethod::with_service_name(service_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JsonRPCMethod;

    #[tokio::test]
    async fn test_healthcheck_method_new() {
        let method = HealthcheckMethod::new();
        assert_eq!(method.method_name(), "healthcheck");
        assert_eq!(method.service_name, "ash-rpc-service");
        assert_eq!(method.version, None);
    }

    #[tokio::test]
    async fn test_healthcheck_method_with_service_name() {
        let method = HealthcheckMethod::with_service_name("custom-service");
        assert_eq!(method.service_name, "custom-service");
        assert_eq!(method.version, None);
    }

    #[tokio::test]
    async fn test_healthcheck_method_service_name_builder() {
        let method = HealthcheckMethod::new().service_name("builder-service");
        assert_eq!(method.service_name, "builder-service");
    }

    #[tokio::test]
    async fn test_healthcheck_method_with_version() {
        let method = HealthcheckMethod::new().with_version("1.0.0");
        assert_eq!(method.version, Some("1.0.0".to_string()));
    }

    #[tokio::test]
    async fn test_healthcheck_method_builder_chain() {
        let method = HealthcheckMethod::new()
            .service_name("my-service")
            .with_version("2.0.0");
        assert_eq!(method.service_name, "my-service");
        assert_eq!(method.version, Some("2.0.0".to_string()));
    }

    #[tokio::test]
    async fn test_healthcheck_default() {
        let method = HealthcheckMethod::default();
        assert_eq!(method.service_name, "ash-rpc-service");
        assert_eq!(method.version, None);
    }

    #[tokio::test]
    async fn test_healthcheck_call_success() {
        let method = HealthcheckMethod::new()
            .service_name("test-service")
            .with_version("1.0.0");

        let id = Some(serde_json::Value::Number(1.into()));
        let response = method.call(None, id.clone()).await;

        // Verify response structure
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert_eq!(response.id, id);

        // Verify health status in result
        let result = response.result.unwrap();
        let status: HealthStatus = serde_json::from_value(result).unwrap();
        assert_eq!(status.status, "healthy");
        assert_eq!(status.service, "test-service");
        assert_eq!(status.version, Some("1.0.0".to_string()));
        assert!(status.timestamp > 0);
    }

    #[tokio::test]
    async fn test_healthcheck_call_with_params() {
        let method = HealthcheckMethod::new();
        let params = Some(serde_json::json!({"ignored": "value"}));
        let id = Some(serde_json::Value::String("test-id".to_string()));

        let response = method.call(params, id).await;

        // Params should be ignored, still returns healthy status
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_healthcheck_call_no_version() {
        let method = HealthcheckMethod::new();
        let response = method.call(None, None).await;

        let result = response.result.unwrap();
        let status: HealthStatus = serde_json::from_value(result).unwrap();
        assert_eq!(status.version, None);
    }

    #[tokio::test]
    async fn test_healthcheck_convenience_function() {
        let method = healthcheck();
        assert_eq!(method.service_name, "ash-rpc-service");
        assert_eq!(method.method_name(), "healthcheck");
    }

    #[tokio::test]
    async fn test_healthcheck_with_service_convenience() {
        let method = healthcheck_with_service("my-service");
        assert_eq!(method.service_name, "my-service");
    }

    #[test]
    fn test_health_status_serialization() {
        let status = HealthStatus {
            status: "healthy".to_string(),
            timestamp: 1234567890,
            service: "test-service".to_string(),
            version: Some("1.0.0".to_string()),
        };

        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["timestamp"], 1234567890);
        assert_eq!(json["service"], "test-service");
        assert_eq!(json["version"], "1.0.0");
    }

    #[test]
    fn test_health_status_deserialization() {
        let json = serde_json::json!({
            "status": "healthy",
            "timestamp": 1234567890,
            "service": "test-service",
            "version": "1.0.0"
        });

        let status: HealthStatus = serde_json::from_value(json).unwrap();
        assert_eq!(status.status, "healthy");
        assert_eq!(status.timestamp, 1234567890);
        assert_eq!(status.service, "test-service");
        assert_eq!(status.version, Some("1.0.0".to_string()));
    }
}
