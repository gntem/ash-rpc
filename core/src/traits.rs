//! Core traits for JSON-RPC handlers and processors.

use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Async trait for individual JSON-RPC method implementations
#[async_trait::async_trait]
pub trait JsonRPCMethod: Send + Sync {
    /// Get the method name that this implementation handles
    fn method_name(&self) -> &'static str;
    
    /// Execute the JSON-RPC method asynchronously
    async fn call(
        &self,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Response;
    
    /// Get OpenAPI components for this method
    fn openapi_components(&self) -> OpenApiMethodSpec {
        OpenApiMethodSpec::new(self.method_name())
    }
}

/// Trait for handling JSON-RPC requests and notifications
#[async_trait::async_trait]
pub trait Handler: Send + Sync {
    /// Handle a JSON-RPC request and return a response
    async fn handle_request(&self, request: Request) -> Response;

    /// Handle a JSON-RPC notification (no response expected)
    async fn handle_notification(&self, notification: Notification);

    /// Check if a method is supported
    fn supports_method(&self, method: &str) -> bool {
        let _ = method;
        true
    }

    /// Get list of supported methods
    fn get_supported_methods(&self) -> Vec<String> {
        vec![]
    }
}

/// Trait for processing JSON-RPC messages
#[async_trait::async_trait]
pub trait MessageProcessor: Send + Sync {
    /// Process a single JSON-RPC message
    async fn process_message(&self, message: Message) -> Option<Response>;

    /// Process a batch of JSON-RPC messages
    async fn process_batch(&self, messages: Vec<Message>) -> Vec<Response> {
        let mut results = Vec::new();
        for msg in messages {
            if let Some(response) = self.process_message(msg).await {
                results.push(response);
            }
        }
        results
    }

    /// Check if batch processing is supported
    fn supports_batching(&self) -> bool {
        true
    }

    /// Get processor capabilities
    fn get_capabilities(&self) -> ProcessorCapabilities {
        ProcessorCapabilities::default()
    }
}

#[derive(Debug, Clone)]
pub struct ProcessorCapabilities {
    pub supports_batch: bool,
    pub supports_notifications: bool,
    pub max_batch_size: Option<usize>,
    pub max_request_size: Option<usize>,
    pub request_timeout_secs: Option<u64>,
    pub supported_versions: Vec<String>,
}

impl Default for ProcessorCapabilities {
    fn default() -> Self {
        Self {
            supports_batch: true,
            supports_notifications: true,
            max_batch_size: Some(100),       // Secure default: limit batch size
            max_request_size: Some(1024 * 1024), // 1 MB
            request_timeout_secs: Some(30),
            supported_versions: vec!["2.0".to_string()],
        }
    }
}

/// Builder for ProcessorCapabilities with validation
pub struct ProcessorCapabilitiesBuilder {
    supports_batch: bool,
    supports_notifications: bool,
    max_batch_size: Option<usize>,
    max_request_size: Option<usize>,
    request_timeout_secs: Option<u64>,
    supported_versions: Vec<String>,
}

impl ProcessorCapabilitiesBuilder {
    /// Create a new builder with secure defaults
    pub fn new() -> Self {
        Self {
            supports_batch: true,
            supports_notifications: true,
            max_batch_size: Some(100),
            max_request_size: Some(1024 * 1024),
            request_timeout_secs: Some(30),
            supported_versions: vec!["2.0".to_string()],
        }
    }

    /// Enable or disable batch support
    pub fn supports_batch(mut self, enabled: bool) -> Self {
        self.supports_batch = enabled;
        self
    }

    /// Enable or disable notification support
    pub fn supports_notifications(mut self, enabled: bool) -> Self {
        self.supports_notifications = enabled;
        self
    }

    /// Set maximum batch size with validation
    /// 
    /// # Arguments
    /// * `size` - Maximum batch size (1-1000), or None for unlimited
    /// 
    /// # Panics
    /// Panics if size is 0 or greater than 1000
    pub fn max_batch_size(mut self, size: Option<usize>) -> Self {
        if let Some(s) = size {
            assert!(s > 0 && s <= 1000, "max_batch_size must be between 1 and 1000");
        }
        self.max_batch_size = size;
        self
    }

    /// Set maximum request size in bytes with validation
    /// 
    /// # Arguments
    /// * `size` - Maximum size in bytes (1KB-100MB), or None for unlimited
    /// 
    /// # Panics
    /// Panics if size is less than 1KB or greater than 100MB
    pub fn max_request_size(mut self, size: Option<usize>) -> Self {
        if let Some(s) = size {
            assert!((1024..=100 * 1024 * 1024).contains(&s),
                    "max_request_size must be between 1KB and 100MB");
        }
        self.max_request_size = size;
        self
    }

    /// Set request timeout in seconds with validation
    /// 
    /// # Arguments
    /// * `timeout` - Timeout in seconds (1-300), or None for no timeout
    /// 
    /// # Panics
    /// Panics if timeout is 0 or greater than 300 seconds
    pub fn request_timeout_secs(mut self, timeout: Option<u64>) -> Self {
        if let Some(t) = timeout {
            assert!(t > 0 && t <= 300, "request_timeout_secs must be between 1 and 300");
        }
        self.request_timeout_secs = timeout;
        self
    }

    /// Add a supported JSON-RPC version
    pub fn add_version(mut self, version: impl Into<String>) -> Self {
        self.supported_versions.push(version.into());
        self
    }

    /// Build the capabilities with validation
    pub fn build(self) -> ProcessorCapabilities {
        tracing::debug!(
            supports_batch = self.supports_batch,
            max_batch_size = ?self.max_batch_size,
            max_request_size = ?self.max_request_size,
            request_timeout_secs = ?self.request_timeout_secs,
            "creating processor capabilities"
        );
        
        ProcessorCapabilities {
            supports_batch: self.supports_batch,
            supports_notifications: self.supports_notifications,
            max_batch_size: self.max_batch_size,
            max_request_size: self.max_request_size,
            request_timeout_secs: self.request_timeout_secs,
            supported_versions: self.supported_versions,
        }
    }
}

impl Default for ProcessorCapabilitiesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// OpenAPI specification for a single JSON-RPC method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiMethodSpec {
    pub method_name: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub errors: Vec<OpenApiError>,
    pub tags: Vec<String>,
    pub examples: Vec<OpenApiExample>,
}

impl OpenApiMethodSpec {
    /// Create a new OpenAPI method specification
    pub fn new(method_name: impl Into<String>) -> Self {
        Self {
            method_name: method_name.into(),
            summary: None,
            description: None,
            parameters: None,
            result: None,
            errors: Vec::new(),
            tags: Vec::new(),
            examples: Vec::new(),
        }
    }

    /// Add a summary
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Add a description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add parameter schema
    pub fn with_parameters(mut self, params: serde_json::Value) -> Self {
        self.parameters = Some(params);
        self
    }

    /// Add result schema
    pub fn with_result(mut self, result: serde_json::Value) -> Self {
        self.result = Some(result);
        self
    }

    /// Add an error specification
    pub fn with_error(mut self, error: OpenApiError) -> Self {
        self.errors.push(error);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add an example
    pub fn with_example(mut self, example: OpenApiExample) -> Self {
        self.examples.push(example);
        self
    }
}

/// OpenAPI error specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiError {
    pub code: i32,
    pub message: String,
    pub description: Option<String>,
}

impl OpenApiError {
    /// Create a new error specification
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            description: None,
        }
    }

    /// Add description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// OpenAPI example for a method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiExample {
    pub name: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub params: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
}

impl OpenApiExample {
    /// Create a new example
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            summary: None,
            description: None,
            params: None,
            result: None,
        }
    }

    /// Add summary
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Add description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add parameters
    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Add result
    pub fn with_result(mut self, result: serde_json::Value) -> Self {
        self.result = Some(result);
        self
    }
}

/// Complete OpenAPI specification composed from all registered methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: OpenApiInfo,
    pub servers: Vec<OpenApiServer>,
    pub methods: HashMap<String, OpenApiMethodSpec>,
    pub components: OpenApiComponents,
}

impl OpenApiSpec {
    /// Create a new OpenAPI specification
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            openapi: "3.0.3".to_string(),
            info: OpenApiInfo {
                title: title.into(),
                version: version.into(),
                description: None,
            },
            servers: Vec::new(),
            methods: HashMap::new(),
            components: OpenApiComponents::default(),
        }
    }

    /// Add a method specification
    pub fn add_method(&mut self, spec: OpenApiMethodSpec) {
        self.methods.insert(spec.method_name.clone(), spec);
    }

    /// Add multiple method specifications
    pub fn add_methods(&mut self, specs: Vec<OpenApiMethodSpec>) {
        for spec in specs {
            self.add_method(spec);
        }
    }

    /// Add a server
    pub fn add_server(&mut self, server: OpenApiServer) {
        self.servers.push(server);
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.info.description = Some(description.into());
        self
    }
}

/// OpenAPI info section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiInfo {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
}

/// OpenAPI server specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiServer {
    pub url: String,
    pub description: Option<String>,
}

impl OpenApiServer {
    /// Create a new server specification
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            description: None,
        }
    }

    /// Add description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// OpenAPI components section
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenApiComponents {
    pub schemas: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ProcessorCapabilities tests
    #[test]
    fn test_processor_capabilities_default() {
        let caps = ProcessorCapabilities::default();
        assert!(caps.supports_batch);
        assert!(caps.supports_notifications);
        assert_eq!(caps.max_batch_size, Some(100));
        assert_eq!(caps.max_request_size, Some(1024 * 1024));
        assert_eq!(caps.request_timeout_secs, Some(30));
        assert_eq!(caps.supported_versions, vec!["2.0"]);
    }

    #[test]
    fn test_processor_capabilities_builder() {
        let caps = ProcessorCapabilitiesBuilder::new()
            .max_batch_size(Some(50))
            .max_request_size(Some(2 * 1024 * 1024))
            .request_timeout_secs(Some(60))
            .build();
        
        assert_eq!(caps.max_batch_size, Some(50));
        assert_eq!(caps.max_request_size, Some(2 * 1024 * 1024));
        assert_eq!(caps.request_timeout_secs, Some(60));
    }

    #[test]
    #[should_panic(expected = "max_batch_size must be between 1 and 1000")]
    fn test_processor_capabilities_invalid_batch_size() {
        ProcessorCapabilitiesBuilder::new()
            .max_batch_size(Some(0))
            .build();
    }

    #[test]
    #[should_panic(expected = "max_batch_size must be between 1 and 1000")]
    fn test_processor_capabilities_batch_size_too_large() {
        ProcessorCapabilitiesBuilder::new()
            .max_batch_size(Some(2000))
            .build();
    }

    #[test]
    fn test_processor_capabilities_builder_boundary() {
        // Test minimum
        let caps_min = ProcessorCapabilitiesBuilder::new()
            .max_batch_size(Some(1))
            .build();
        assert_eq!(caps_min.max_batch_size, Some(1));

        // Test maximum
        let caps_max = ProcessorCapabilitiesBuilder::new()
            .max_batch_size(Some(1000))
            .build();
        assert_eq!(caps_max.max_batch_size, Some(1000));
    }

    // OpenApiMethodSpec tests
    #[test]
    fn test_openapi_method_spec_creation() {
        let spec = OpenApiMethodSpec::new("test_method");
        assert_eq!(spec.method_name, "test_method");
        assert!(spec.description.is_none());
        assert!(spec.parameters.is_none());
        assert!(spec.result.is_none());
    }

    #[test]
    fn test_openapi_method_spec_with_description() {
        let spec = OpenApiMethodSpec::new("method")
            .with_description("Test description");
        assert_eq!(spec.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_openapi_method_spec_with_schemas() {
        let params = json!({"type": "object"});
        let result = json!({"type": "string"});
        
        let spec = OpenApiMethodSpec::new("method")
            .with_parameters(params.clone())
            .with_result(result.clone());
        
        assert_eq!(spec.parameters, Some(params));
        assert_eq!(spec.result, Some(result));
    }

    #[test]
    fn test_openapi_method_spec_complete() {
        let spec = OpenApiMethodSpec::new("complete_method")
            .with_description("A complete method")
            .with_parameters(json!({"type": "array"}))
            .with_result(json!({"type": "number"}));
        
        assert_eq!(spec.method_name, "complete_method");
        assert_eq!(spec.description, Some("A complete method".to_string()));
        assert!(spec.parameters.is_some());
        assert!(spec.result.is_some());
    }

    // OpenApiSpec tests
    #[test]
    fn test_openapi_spec_creation() {
        let spec = OpenApiSpec::new("Test API", "1.0.0");
        assert_eq!(spec.openapi, "3.0.3");
        assert_eq!(spec.info.title, "Test API");
        assert_eq!(spec.info.version, "1.0.0");
    }

    #[test]
    fn test_openapi_spec_add_server() {
        let mut spec = OpenApiSpec::new("API", "1.0.0");
        spec.add_server(OpenApiServer::new("http://localhost:8080"));
        assert_eq!(spec.servers.len(), 1);
        assert_eq!(spec.servers[0].url, "http://localhost:8080");
    }

    #[test]
    fn test_openapi_spec_add_method() {
        let mut spec = OpenApiSpec::new("API", "1.0.0");
        let method_spec = OpenApiMethodSpec::new("test");
        spec.add_method(method_spec);
        assert_eq!(spec.methods.len(), 1);
    }

    #[test]
    fn test_openapi_spec_serialization() {
        let mut spec = OpenApiSpec::new("Test", "1.0");
        spec.add_server(OpenApiServer::new("http://api.example.com"));
        
        let json = serde_json::to_string(&spec).unwrap();
        let deserialized: OpenApiSpec = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.info.title, "Test");
        assert_eq!(deserialized.servers.len(), 1);
    }

    // OpenApiServer tests
    #[test]
    fn test_openapi_server_creation() {
        let server = OpenApiServer::new("http://localhost:3000");
        assert_eq!(server.url, "http://localhost:3000");
        assert!(server.description.is_none());
    }

    #[test]
    fn test_openapi_server_with_description() {
        let server = OpenApiServer::new("http://api.com")
            .with_description("Production API");
        assert_eq!(server.description, Some("Production API".to_string()));
    }

    // OpenApiInfo tests
    #[test]
    fn test_openapi_info_creation() {
        let info = OpenApiInfo {
            title: "My API".to_string(),
            version: "2.0.0".to_string(),
            description: Some("API description".to_string()),
        };
        
        assert_eq!(info.title, "My API");
        assert_eq!(info.version, "2.0.0");
        assert_eq!(info.description, Some("API description".to_string()));
    }

    // OpenApiComponents tests
    #[test]
    fn test_openapi_components_default() {
        let components = OpenApiComponents::default();
        assert_eq!(components.schemas.len(), 0);
    }

    #[test]
    fn test_openapi_components_with_schemas() {
        let mut components = OpenApiComponents::default();
        components.schemas.insert(
            "User".to_string(),
            json!({"type": "object", "properties": {"name": {"type": "string"}}}),
        );
        
        assert_eq!(components.schemas.len(), 1);
        assert!(components.schemas.contains_key("User"));
    }

    // Test JsonRPCMethod trait implementation
    struct TestMethod;

    #[async_trait::async_trait]
    impl JsonRPCMethod for TestMethod {
        fn method_name(&self) -> &'static str {
            "test"
        }

        async fn call(
            &self,
            params: Option<serde_json::Value>,
            id: Option<RequestId>,
        ) -> Response {
            Response::success(params.unwrap_or(json!(null)), id)
        }
    }

    #[tokio::test]
    async fn test_jsonrpc_method_trait() {
        let method = TestMethod;
        assert_eq!(method.method_name(), "test");
        
        let params = json!({"key": "value"});
        let response = method.call(Some(params.clone()), Some(json!(1))).await;
        
        assert!(response.is_success());
        assert_eq!(response.result, Some(params));
    }

    #[tokio::test]
    async fn test_jsonrpc_method_openapi_components() {
        let method = TestMethod;
        let spec = method.openapi_components();
        assert_eq!(spec.method_name, "test");
    }

    // Handler trait tests with mock implementation
    struct TestHandler;

    #[async_trait::async_trait]
    impl Handler for TestHandler {
        async fn handle_request(&self, request: Request) -> Response {
            Response::success(json!({"handled": request.method}), request.id)
        }

        async fn handle_notification(&self, _notification: Notification) {
            // No-op
        }

        fn supports_method(&self, method: &str) -> bool {
            method == "supported"
        }

        fn get_supported_methods(&self) -> Vec<String> {
            vec!["supported".to_string(), "another".to_string()]
        }
    }

    #[tokio::test]
    async fn test_handler_handle_request() {
        let handler = TestHandler;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "test".to_string(),
            params: None,
            id: Some(json!(1)),
            correlation_id: None,
        };

        let response = handler.handle_request(request).await;
        assert!(response.is_success());
    }

    #[tokio::test]
    async fn test_handler_supports_method() {
        let handler = TestHandler;
        assert!(handler.supports_method("supported"));
        assert!(!handler.supports_method("unsupported"));
    }

    #[tokio::test]
    async fn test_handler_get_supported_methods() {
        let handler = TestHandler;
        let methods = handler.get_supported_methods();
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&"supported".to_string()));
        assert!(methods.contains(&"another".to_string()));
    }

    // MessageProcessor trait tests
    struct TestProcessor;

    #[async_trait::async_trait]
    impl MessageProcessor for TestProcessor {
        async fn process_message(&self, message: Message) -> Option<Response> {
            match message {
                Message::Request(req) => Some(Response::success(json!("ok"), req.id)),
                _ => None,
            }
        }
    }

    #[tokio::test]
    async fn test_message_processor_single_message() {
        let processor = TestProcessor;
        let request = Message::Request(Request {
            jsonrpc: "2.0".to_string(),
            method: "test".to_string(),
            params: None,
            id: Some(json!(1)),
            correlation_id: None,
        });

        let response = processor.process_message(request).await;
        assert!(response.is_some());
    }

    #[tokio::test]
    async fn test_message_processor_batch() {
        let processor = TestProcessor;
        let messages = vec![
            Message::Request(Request {
                jsonrpc: "2.0".to_string(),
                method: "test1".to_string(),
                params: None,
                id: Some(json!(1)),
                correlation_id: None,
            }),
            Message::Request(Request {
                jsonrpc: "2.0".to_string(),
                method: "test2".to_string(),
                params: None,
                id: Some(json!(2)),
                correlation_id: None,
            }),
        ];

        let responses = processor.process_batch(messages).await;
        assert_eq!(responses.len(), 2);
    }

    #[tokio::test]
    async fn test_message_processor_supports_batching() {
        let processor = TestProcessor;
        assert!(processor.supports_batching());
    }

    #[tokio::test]
    async fn test_message_processor_capabilities() {
        let processor = TestProcessor;
        let caps = processor.get_capabilities();
        assert!(caps.supports_batch);
        assert!(caps.supports_notifications);
    }

    // ProcessorCapabilities additional tests
    #[test]
    fn test_processor_capabilities_builder_disabled_batch() {
        let caps = ProcessorCapabilitiesBuilder::new()
            .supports_batch(false)
            .build();
        assert!(!caps.supports_batch);
    }

    #[test]
    fn test_processor_capabilities_builder_disabled_notifications() {
        let caps = ProcessorCapabilitiesBuilder::new()
            .supports_notifications(false)
            .build();
        assert!(!caps.supports_notifications);
    }

    #[test]
    fn test_processor_capabilities_builder_add_version() {
        let caps = ProcessorCapabilitiesBuilder::new()
            .add_version("3.0")
            .build();
        assert!(caps.supported_versions.contains(&"2.0".to_string()));
        assert!(caps.supported_versions.contains(&"3.0".to_string()));
    }

    #[test]
    fn test_processor_capabilities_builder_none_limits() {
        let caps = ProcessorCapabilitiesBuilder::new()
            .max_batch_size(None)
            .max_request_size(None)
            .request_timeout_secs(None)
            .build();
        
        assert!(caps.max_batch_size.is_none());
        assert!(caps.max_request_size.is_none());
        assert!(caps.request_timeout_secs.is_none());
    }

    // OpenApiError tests
    #[test]
    fn test_openapi_error_creation() {
        let error = OpenApiError::new(-32600, "Invalid Request");
        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Invalid Request");
        assert!(error.description.is_none());
    }

    #[test]
    fn test_openapi_error_with_description() {
        let error = OpenApiError::new(-32600, "Invalid Request")
            .with_description("The JSON sent is not a valid Request object");
        assert_eq!(error.description, Some("The JSON sent is not a valid Request object".to_string()));
    }

    // OpenApiExample tests
    #[test]
    fn test_openapi_example_creation() {
        let example = OpenApiExample::new("basic_example");
        assert_eq!(example.name, "basic_example");
        assert!(example.summary.is_none());
        assert!(example.description.is_none());
    }

    #[test]
    fn test_openapi_example_complete() {
        let example = OpenApiExample::new("complete")
            .with_summary("Complete example")
            .with_description("A complete example with all fields")
            .with_params(json!({"x": 1, "y": 2}))
            .with_result(json!(3));
        
        assert_eq!(example.summary, Some("Complete example".to_string()));
        assert_eq!(example.description, Some("A complete example with all fields".to_string()));
        assert!(example.params.is_some());
        assert!(example.result.is_some());
    }

    // OpenApiMethodSpec additional tests
    #[test]
    fn test_openapi_method_spec_with_error() {
        let error = OpenApiError::new(-32602, "Invalid params");
        let spec = OpenApiMethodSpec::new("method")
            .with_error(error);
        
        assert_eq!(spec.errors.len(), 1);
        assert_eq!(spec.errors[0].code, -32602);
    }

    #[test]
    fn test_openapi_method_spec_with_tag() {
        let spec = OpenApiMethodSpec::new("method")
            .with_tag("utility")
            .with_tag("public");
        
        assert_eq!(spec.tags.len(), 2);
        assert!(spec.tags.contains(&"utility".to_string()));
        assert!(spec.tags.contains(&"public".to_string()));
    }

    #[test]
    fn test_openapi_method_spec_with_example() {
        let example = OpenApiExample::new("example1");
        let spec = OpenApiMethodSpec::new("method")
            .with_example(example);
        
        assert_eq!(spec.examples.len(), 1);
        assert_eq!(spec.examples[0].name, "example1");
    }

    #[test]
    fn test_openapi_method_spec_with_summary() {
        let spec = OpenApiMethodSpec::new("method")
            .with_summary("Method summary");
        assert_eq!(spec.summary, Some("Method summary".to_string()));
    }

    // OpenApiSpec additional tests
    #[test]
    fn test_openapi_spec_add_multiple_methods() {
        let mut spec = OpenApiSpec::new("API", "1.0");
        let methods = vec![
            OpenApiMethodSpec::new("method1"),
            OpenApiMethodSpec::new("method2"),
            OpenApiMethodSpec::new("method3"),
        ];
        
        spec.add_methods(methods);
        assert_eq!(spec.methods.len(), 3);
    }

    #[test]
    fn test_openapi_spec_with_description() {
        let spec = OpenApiSpec::new("API", "1.0")
            .with_description("Test API description");
        assert_eq!(spec.info.description, Some("Test API description".to_string()));
    }

    #[test]
    fn test_openapi_spec_multiple_servers() {
        let mut spec = OpenApiSpec::new("API", "1.0");
        spec.add_server(OpenApiServer::new("http://dev.example.com"));
        spec.add_server(OpenApiServer::new("https://prod.example.com"));
        
        assert_eq!(spec.servers.len(), 2);
    }
}
