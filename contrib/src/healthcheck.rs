//! Health check JSON-RPC method implementation
//!
//! Provides a standard health check method that can be used to monitor
//! service availability and health status.

use ash_rpc_core::*;
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

#[ash_rpc_core::async_trait]
impl JsonRPCMethod for HealthcheckMethod {
    fn method_name(&self) -> &'static str {
        "healthcheck"
    }
    
    async fn call(
        &self,
        _params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Response {
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