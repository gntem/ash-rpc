//! Builder patterns for JSON-RPC types.

use crate::types::*;

/// Builder for JSON-RPC requests
pub struct RequestBuilder {
    method: String,
    params: Option<serde_json::Value>,
    id: Option<RequestId>,
    correlation_id: Option<String>,
}

impl RequestBuilder {
    /// Create a new request builder
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            params: None,
            id: None,
            correlation_id: Some(uuid::Uuid::new_v4().to_string()),
        }
    }

    /// Set request parameters
    pub fn params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Set request ID
    pub fn id(mut self, id: RequestId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set correlation ID
    pub fn correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Build the request
    pub fn build(self) -> Request {
        Request {
            jsonrpc: "2.0".to_string(),
            method: self.method,
            params: self.params,
            id: self.id,
            correlation_id: self.correlation_id,
        }
    }
}

/// Builder for JSON-RPC responses
pub struct ResponseBuilder {
    result: Option<serde_json::Value>,
    error: Option<Error>,
    id: Option<RequestId>,
    correlation_id: Option<String>,
}

impl ResponseBuilder {
    /// Create a new response builder
    pub fn new() -> Self {
        Self {
            result: None,
            error: None,
            id: None,
            correlation_id: None,
        }
    }

    /// Set successful result
    pub fn success(mut self, result: serde_json::Value) -> Self {
        self.result = Some(result);
        self
    }

    /// Set error
    pub fn error(mut self, error: Error) -> Self {
        self.error = Some(error);
        self
    }

    /// Set response ID
    pub fn id(mut self, id: Option<RequestId>) -> Self {
        self.id = id;
        self
    }
    /// Set correlation ID
    pub fn correlation_id(mut self, correlation_id: Option<String>) -> Self {
        self.correlation_id = correlation_id;
        self
    }
    /// Build the response
    pub fn build(self) -> Response {
        Response {
            jsonrpc: "2.0".to_string(),
            result: self.result,
            error: self.error,
            id: self.id,
            correlation_id: self.correlation_id,
        }
    }
}

/// Builder for JSON-RPC notifications
pub struct NotificationBuilder {
    method: String,
    params: Option<serde_json::Value>,
}

impl NotificationBuilder {
    /// Create a new notification builder
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            params: None,
        }
    }

    /// Set notification parameters
    pub fn params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Build the notification
    pub fn build(self) -> Notification {
        Notification {
            jsonrpc: "2.0".to_string(),
            method: self.method,
            params: self.params,
        }
    }
}

/// Builder for JSON-RPC errors
pub struct ErrorBuilder {
    code: i32,
    message: String,
    data: Option<serde_json::Value>,
}

impl ErrorBuilder {
    /// Create a new error builder
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Add additional error data
    pub fn data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Build the error
    pub fn build(self) -> Error {
        Error {
            code: self.code,
            message: self.message,
            data: self.data,
        }
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for security configuration with validation
#[cfg(any(feature = "tcp", feature = "tcp-stream", feature = "tcp-stream-tls"))]
pub struct SecurityConfigBuilder {
    max_connections: usize,
    max_request_size: usize,
    request_timeout: std::time::Duration,
    idle_timeout: std::time::Duration,
}

#[cfg(any(feature = "tcp", feature = "tcp-stream", feature = "tcp-stream-tls"))]
impl SecurityConfigBuilder {
    /// Create a new security config builder with secure defaults
    pub fn new() -> Self {
        Self {
            max_connections: 1000,
            max_request_size: 1024 * 1024, // 1 MB
            request_timeout: std::time::Duration::from_secs(30),
            idle_timeout: std::time::Duration::from_secs(300), // 5 minutes
        }
    }

    /// Set maximum concurrent connections
    ///
    /// # Arguments
    /// * `max` - Maximum number of connections (1-100000)
    ///
    /// # Panics
    /// Panics if max is 0 or greater than 100000
    pub fn max_connections(mut self, max: usize) -> Self {
        assert!(
            max > 0 && max <= 100_000,
            "max_connections must be between 1 and 100000"
        );
        self.max_connections = max;
        self
    }

    /// Set maximum request size in bytes
    ///
    /// # Arguments
    /// * `size` - Maximum size in bytes (1024 to 100MB)
    ///
    /// # Panics
    /// Panics if size is less than 1024 bytes or greater than 100MB
    pub fn max_request_size(mut self, size: usize) -> Self {
        assert!(
            (1024..=100 * 1024 * 1024).contains(&size),
            "max_request_size must be between 1KB and 100MB"
        );
        self.max_request_size = size;
        self
    }

    /// Set request timeout
    ///
    /// # Arguments
    /// * `timeout` - Timeout duration (1 second to 5 minutes)
    ///
    /// # Panics
    /// Panics if timeout is less than 1 second or greater than 5 minutes
    pub fn request_timeout(mut self, timeout: std::time::Duration) -> Self {
        assert!(
            (1..=300).contains(&timeout.as_secs()),
            "request_timeout must be between 1 second and 5 minutes"
        );
        self.request_timeout = timeout;
        self
    }

    /// Set idle connection timeout
    ///
    /// # Arguments
    /// * `timeout` - Timeout duration (10 seconds to 1 hour)
    ///
    /// # Panics
    /// Panics if timeout is less than 10 seconds or greater than 1 hour
    pub fn idle_timeout(mut self, timeout: std::time::Duration) -> Self {
        assert!(
            (10..=3600).contains(&timeout.as_secs()),
            "idle_timeout must be between 10 seconds and 1 hour"
        );
        self.idle_timeout = timeout;
        self
    }

    /// Build the security configuration with validation
    pub fn build(self) -> crate::transport::SecurityConfig {
        tracing::info!(
            max_connections = self.max_connections,
            max_request_size = self.max_request_size,
            request_timeout_secs = self.request_timeout.as_secs(),
            idle_timeout_secs = self.idle_timeout.as_secs(),
            "creating security configuration"
        );

        crate::transport::SecurityConfig {
            max_connections: self.max_connections,
            max_request_size: self.max_request_size,
            request_timeout: self.request_timeout,
            idle_timeout: self.idle_timeout,
        }
    }
}

#[cfg(any(feature = "tcp", feature = "tcp-stream", feature = "tcp-stream-tls"))]
impl Default for SecurityConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(
    test,
    any(feature = "tcp", feature = "tcp-stream", feature = "tcp-stream-tls")
))]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "max_connections must be between 1 and 100000")]
    fn test_max_connections_zero_panics() {
        SecurityConfigBuilder::new().max_connections(0).build();
    }

    #[test]
    fn test_valid_security_config() {
        let config = SecurityConfigBuilder::new()
            .max_connections(500)
            .max_request_size(2 * 1024 * 1024)
            .request_timeout(std::time::Duration::from_secs(60))
            .idle_timeout(std::time::Duration::from_secs(600))
            .build();

        assert_eq!(config.max_connections, 500);
        assert_eq!(config.max_request_size, 2 * 1024 * 1024);
    }

    // RequestBuilder tests
    #[test]
    fn test_request_builder_basic() {
        let request = RequestBuilder::new("test_method").build();
        assert_eq!(request.method, "test_method");
        assert_eq!(request.jsonrpc, "2.0");
        assert!(request.correlation_id.is_some());
    }

    #[test]
    fn test_request_builder_with_params() {
        let params = serde_json::json!({"key": "value"});
        let request = RequestBuilder::new("method").params(params.clone()).build();
        assert_eq!(request.params, Some(params));
    }

    #[test]
    fn test_request_builder_with_id() {
        let id = serde_json::json!(123);
        let request = RequestBuilder::new("method").id(id.clone()).build();
        assert_eq!(request.id, Some(id));
    }

    #[test]
    fn test_request_builder_with_correlation_id() {
        let correlation_id = "custom-correlation-id".to_string();
        let request = RequestBuilder::new("method")
            .correlation_id(correlation_id.clone())
            .build();
        assert_eq!(request.correlation_id, Some(correlation_id));
    }

    #[test]
    fn test_request_builder_complete() {
        let params = serde_json::json!([1, 2, 3]);
        let id = serde_json::json!(456);
        let correlation_id = "test-corr-id".to_string();

        let request = RequestBuilder::new("complete_method")
            .params(params.clone())
            .id(id.clone())
            .correlation_id(correlation_id.clone())
            .build();

        assert_eq!(request.method, "complete_method");
        assert_eq!(request.params, Some(params));
        assert_eq!(request.id, Some(id));
        assert_eq!(request.correlation_id, Some(correlation_id));
    }

    // ResponseBuilder tests
    #[test]
    fn test_response_builder_success() {
        let result = serde_json::json!({"status": "ok"});
        let id = serde_json::json!(1);

        let response = ResponseBuilder::new()
            .success(result.clone())
            .id(Some(id.clone()))
            .build();

        assert_eq!(response.result, Some(result));
        assert!(response.error.is_none());
        assert_eq!(response.id, Some(id));
    }

    #[test]
    fn test_response_builder_error() {
        let error = crate::Error::new(-32600, "Invalid request");
        let id = serde_json::json!(2);

        let response = ResponseBuilder::new()
            .error(error.clone())
            .id(Some(id.clone()))
            .build();

        assert!(response.result.is_none());
        assert_eq!(response.error.unwrap().code, error.code);
        assert_eq!(response.id, Some(id));
    }

    #[test]
    fn test_response_builder_with_correlation_id_basic() {
        let correlation_id = "resp-corr-id".to_string();
        let response = ResponseBuilder::new()
            .success(serde_json::json!("ok"))
            .correlation_id(Some(correlation_id.clone()))
            .build();

        assert_eq!(response.correlation_id, Some(correlation_id));
    }

    #[test]
    fn test_response_builder_jsonrpc_version() {
        let response = ResponseBuilder::new()
            .success(serde_json::json!("test"))
            .build();
        assert_eq!(response.jsonrpc, "2.0");
    }

    // NotificationBuilder tests
    #[test]
    fn test_notification_builder_basic() {
        let notification = NotificationBuilder::new("event").build();
        assert_eq!(notification.method, "event");
        assert_eq!(notification.jsonrpc, "2.0");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_notification_builder_with_params() {
        let params = serde_json::json!({"event": "update"});
        let notification = NotificationBuilder::new("notify")
            .params(params.clone())
            .build();
        assert_eq!(notification.params, Some(params));
    }

    // ErrorBuilder tests
    #[test]
    fn test_error_builder_basic() {
        let error = ErrorBuilder::new(-32600, "Test error").build();
        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Test error");
        assert!(error.data.is_none());
    }

    #[test]
    fn test_error_builder_with_data() {
        let data = serde_json::json!({"detail": "more info"});
        let error = ErrorBuilder::new(-32000, "Error")
            .data(data.clone())
            .build();
        assert_eq!(error.data, Some(data));
    }

    #[test]
    fn test_error_builder_string_conversion() {
        let error = ErrorBuilder::new(-32603, String::from("Dynamic error")).build();
        assert_eq!(error.message, "Dynamic error");
    }

    // SecurityConfigBuilder validation tests
    #[test]
    #[should_panic(expected = "max_request_size must be between")]
    fn test_security_config_request_size_too_small() {
        SecurityConfigBuilder::new()
            .max_request_size(512) // Less than 1KB
            .build();
    }

    #[test]
    #[should_panic(expected = "max_request_size must be between")]
    fn test_security_config_request_size_too_large() {
        SecurityConfigBuilder::new()
            .max_request_size(200 * 1024 * 1024) // More than 100MB
            .build();
    }

    #[test]
    #[should_panic(expected = "max_connections must be between")]
    fn test_security_config_connections_too_large() {
        SecurityConfigBuilder::new()
            .max_connections(150_000)
            .build();
    }

    #[test]
    fn test_security_config_boundary_values() {
        // Test minimum valid values
        let config_min = SecurityConfigBuilder::new()
            .max_connections(1)
            .max_request_size(1024)
            .build();
        assert_eq!(config_min.max_connections, 1);
        assert_eq!(config_min.max_request_size, 1024);

        // Test maximum valid values
        let config_max = SecurityConfigBuilder::new()
            .max_connections(100_000)
            .max_request_size(100 * 1024 * 1024)
            .build();
        assert_eq!(config_max.max_connections, 100_000);
        assert_eq!(config_max.max_request_size, 100 * 1024 * 1024);
    }

    #[test]
    fn test_security_config_timeouts() {
        let request_timeout = std::time::Duration::from_secs(45);
        let idle_timeout = std::time::Duration::from_secs(900);

        let config = SecurityConfigBuilder::new()
            .request_timeout(request_timeout)
            .idle_timeout(idle_timeout)
            .build();

        assert_eq!(config.request_timeout, request_timeout);
        assert_eq!(config.idle_timeout, idle_timeout);
    }

    #[test]
    fn test_security_config_defaults() {
        let config = SecurityConfigBuilder::new().build();
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.max_request_size, 1024 * 1024);
        assert_eq!(config.request_timeout, std::time::Duration::from_secs(30));
        assert_eq!(config.idle_timeout, std::time::Duration::from_secs(300));
    }

    #[test]
    fn test_security_config_builder_default() {
        let builder = SecurityConfigBuilder::default();
        let config = builder.build();
        assert_eq!(config.max_connections, 1000);
    }

    #[test]
    #[should_panic(expected = "request_timeout must be between")]
    fn test_security_config_timeout_too_short() {
        SecurityConfigBuilder::new()
            .request_timeout(std::time::Duration::from_millis(500))
            .build();
    }

    #[test]
    #[should_panic(expected = "request_timeout must be between")]
    fn test_security_config_timeout_too_long() {
        SecurityConfigBuilder::new()
            .request_timeout(std::time::Duration::from_secs(400))
            .build();
    }

    #[test]
    #[should_panic(expected = "idle_timeout must be between")]
    fn test_security_config_idle_timeout_too_short() {
        SecurityConfigBuilder::new()
            .idle_timeout(std::time::Duration::from_secs(5))
            .build();
    }

    #[test]
    #[should_panic(expected = "idle_timeout must be between")]
    fn test_security_config_idle_timeout_too_long() {
        SecurityConfigBuilder::new()
            .idle_timeout(std::time::Duration::from_secs(4000))
            .build();
    }

    #[test]
    fn test_request_builder_method_set() {
        let request = RequestBuilder::new("test_method").build();
        assert_eq!(request.method, "test_method");
    }

    #[test]
    fn test_request_builder_auto_correlation_id() {
        let request = RequestBuilder::new("test").build();
        assert!(request.correlation_id.is_some());
    }

    #[test]
    fn test_request_builder_custom_correlation_id() {
        let custom_id = "custom-correlation-123".to_string();
        let request = RequestBuilder::new("test")
            .correlation_id(custom_id.clone())
            .build();
        assert_eq!(request.correlation_id, Some(custom_id));
    }

    #[test]
    fn test_request_builder_full_chain() {
        let request = RequestBuilder::new("full_test")
            .params(serde_json::json!({"key": "value"}))
            .id(serde_json::json!(123))
            .correlation_id("corr-123".to_string())
            .build();

        assert_eq!(request.method, "full_test");
        assert!(request.params.is_some());
        assert_eq!(request.id, Some(serde_json::json!(123)));
        assert_eq!(request.correlation_id, Some("corr-123".to_string()));
    }

    #[test]
    fn test_response_builder_default_trait() {
        let builder = ResponseBuilder::default();
        let response = builder.build();
        assert_eq!(response.jsonrpc, "2.0");
    }

    #[test]
    fn test_response_builder_success_with_null() {
        let response = ResponseBuilder::new()
            .success(serde_json::json!(null))
            .build();
        assert_eq!(response.result, Some(serde_json::json!(null)));
    }

    #[test]
    fn test_response_builder_with_correlation_id() {
        let corr_id = "test-correlation".to_string();
        let response = ResponseBuilder::new()
            .success(serde_json::json!(42))
            .correlation_id(Some(corr_id.clone()))
            .build();
        assert_eq!(response.correlation_id, Some(corr_id));
    }

    #[test]
    fn test_response_builder_full_error() {
        let error = ErrorBuilder::new(-32001, "Custom error")
            .data(serde_json::json!({"field": "value"}))
            .build();

        let response = ResponseBuilder::new()
            .error(error.clone())
            .id(Some(serde_json::json!(1)))
            .correlation_id(Some("err-corr".to_string()))
            .build();

        assert!(response.result.is_none());
        assert_eq!(response.error, Some(error));
        assert_eq!(response.id, Some(serde_json::json!(1)));
        assert_eq!(response.correlation_id, Some("err-corr".to_string()));
    }

    #[test]
    fn test_notification_builder_string_method() {
        let notification = NotificationBuilder::new(String::from("dynamic_method")).build();
        assert_eq!(notification.method, "dynamic_method");
    }

    #[test]
    fn test_error_builder_multiple_data() {
        let error = ErrorBuilder::new(-32000, "Error")
            .data(serde_json::json!({"key1": "value1"}))
            .build();

        assert!(error.data.is_some());
        let data = error.data.unwrap();
        assert_eq!(data["key1"], "value1");
    }
}

/// Builder for streaming response
#[cfg(feature = "streaming")]
pub struct StreamResponseBuilder {
    result: Option<serde_json::Value>,
    error: Option<Error>,
    id: RequestId,
    stream_id: String,
    stream_status: Option<crate::streaming::StreamStatus>,
}

#[cfg(feature = "streaming")]
impl StreamResponseBuilder {
    /// Create a new stream response builder
    pub fn new(stream_id: impl Into<String>, id: RequestId) -> Self {
        Self {
            result: None,
            error: None,
            id,
            stream_id: stream_id.into(),
            stream_status: None,
        }
    }

    /// Set successful result
    pub fn success(mut self, result: serde_json::Value) -> Self {
        self.result = Some(result);
        self.stream_status = Some(crate::streaming::StreamStatus::Active);
        self
    }

    /// Set error
    pub fn error(mut self, error: Error) -> Self {
        self.error = Some(error);
        self.stream_status = Some(crate::streaming::StreamStatus::Error);
        self
    }

    /// Set stream status
    pub fn status(mut self, status: crate::streaming::StreamStatus) -> Self {
        self.stream_status = Some(status);
        self
    }

    /// Build the stream response
    pub fn build(self) -> crate::streaming::StreamResponse {
        crate::streaming::StreamResponse {
            jsonrpc: "2.0".to_string(),
            result: self.result,
            error: self.error,
            id: self.id,
            stream_id: self.stream_id,
            stream_status: self.stream_status,
        }
    }
}

