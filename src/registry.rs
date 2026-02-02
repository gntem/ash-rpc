//! Method registry for organizing and dispatching JSON-RPC methods.
//!
//! ## Usage
//!
//! ### Basic Usage (Runtime Dispatch)
//! Create method implementations using the `JsonRPCMethod` trait:
//!
//! ```rust
//! use ash_rpc::*;
//!
//! struct PingMethod;
//!
//! #[async_trait::async_trait]
//! impl JsonRPCMethod for PingMethod {
//!     fn method_name(&self) -> &'static str { "ping" }
//!     
//!     async fn call(
//!         &self,
//!         _params: Option<serde_json::Value>,
//!         id: Option<RequestId>,
//!     ) -> Response {
//!         rpc_success!("pong", id)
//!     }
//! }
//!
//! let registry = MethodRegistry::new(register_methods![PingMethod]);
//! ```
//!
//! ### Optimized Usage (Compile-time Dispatch)
//! For better performance, use the dispatch_call! macro:
//!
//! ```text
//! // In your handler function:
//! async fn handle_call(method_name: &str, params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
//!     dispatch_call!(method_name, params, id => PingMethod, EchoMethod, CalculatorMethod)
//! }
//! ```

use crate::builders::*;
use crate::traits::*;
use crate::types::*;
use std::sync::Arc;

/// Method registry with optional authentication
pub struct MethodRegistry {
    methods: Vec<Box<dyn JsonRPCMethod>>,
    auth_policy: Option<Arc<dyn crate::auth::AuthPolicy>>,
}

/// Macro to generate method dispatch match arms for registered JsonRPCMethod implementations
#[macro_export]
macro_rules! register_methods {
    ($($method:expr),* $(,)?) => {
        vec![
            $(
                Box::new($method) as Box<dyn JsonRPCMethod>
            ),*
        ]
    };
}

/// Macro to generate a dispatch function with compile-time method matching
/// This replaces runtime iteration with a compile-time generated match statement
#[macro_export]
macro_rules! dispatch_call {
    ($method_name:expr, $params:expr, $id:expr => $($method:expr),* $(,)?) => {
        {
            // Create temporary instances for method name comparison
            $(
                let temp_method = $method;
                if $method_name == temp_method.method_name() {
                    return temp_method.call($params, $id).await;
                }
            )*

            // Method not found
            ResponseBuilder::new()
                .error(ErrorBuilder::new(error_codes::METHOD_NOT_FOUND, "Method not found").build())
                .id($id)
                .build()
        }
    };
}

impl MethodRegistry {
    /// Create a new method registry with the given method implementations
    pub fn new(methods: Vec<Box<dyn JsonRPCMethod>>) -> Self {
        tracing::debug!(method_count = methods.len(), "registry created");
        Self {
            methods,
            auth_policy: None,
        }
    }

    /// Create an empty registry
    pub fn empty() -> Self {
        Self {
            methods: Vec::new(),
            auth_policy: None,
        }
    }

    /// Set an authentication/authorization policy
    ///
    /// When set, `can_access` will be checked before executing methods.
    /// The user implements ALL auth logic in the trait.
    ///
    /// # Example
    /// ```text
    /// let registry = MethodRegistry::new(methods)
    ///     .with_auth(MyAuthPolicy::new());
    /// ```
    pub fn with_auth<A: crate::auth::AuthPolicy + 'static>(mut self, policy: A) -> Self {
        self.auth_policy = Some(Arc::new(policy));
        self
    }

    /// Add a method implementation to the registry
    pub fn add_method(mut self, method: Box<dyn JsonRPCMethod>) -> Self {
        tracing::trace!("adding method to registry");
        self.methods.push(method);
        self
    }

    /// Call a registered method asynchronously using compile-time dispatch
    /// Note: This method should typically be replaced by using the dispatch_methods! macro directly
    /// for better compile-time optimization
    pub async fn call(
        &self,
        method_name: &str,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Response {
        self.call_with_context(
            method_name,
            params,
            id,
            &crate::auth::ConnectionContext::default(),
        )
        .await
    }

    /// Call a registered method with authentication context
    ///
    /// Use this when you have connection context from your transport layer.
    pub async fn call_with_context(
        &self,
        method_name: &str,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
        ctx: &crate::auth::ConnectionContext,
    ) -> Response {
        // Check authentication if policy is set
        if let Some(auth) = &self.auth_policy
            && !auth.can_access(method_name, params.as_ref(), ctx)
        {
            tracing::warn!(
                method = %method_name,
                remote_addr = ?ctx.remote_addr,
                "access denied by auth policy"
            );
            return auth.unauthorized_error(method_name);
        }

        // Fallback to runtime dispatch if compile-time dispatch is not used
        for method in &self.methods {
            if method.method_name() == method_name {
                tracing::debug!(method = %method_name, "calling method");
                return method.call(params, id).await;
            }
        }

        tracing::warn!(method = %method_name, "method not found");
        ResponseBuilder::new()
            .error(ErrorBuilder::new(error_codes::METHOD_NOT_FOUND, "Method not found").build())
            .id(id)
            .build()
    }

    /// Check if a method is registered
    pub fn has_method(&self, method_name: &str) -> bool {
        self.methods.iter().any(|m| m.method_name() == method_name)
    }

    /// Get list of all registered methods
    pub fn get_methods(&self) -> Vec<String> {
        self.methods
            .iter()
            .map(|m| m.method_name().to_string())
            .collect()
    }

    /// Get the number of registered methods
    pub fn method_count(&self) -> usize {
        self.methods.len()
    }

    /// Generate OpenAPI specification for all registered methods
    pub fn generate_openapi_spec(&self, title: &str, version: &str) -> OpenApiSpec {
        tracing::debug!(method_count = self.methods.len(), "generating openapi spec");
        let mut spec = OpenApiSpec::new(title, version);

        for method in &self.methods {
            let method_spec = method.openapi_components();
            spec.add_method(method_spec);
        }

        spec
    }

    /// Generate OpenAPI specification with custom info and servers
    pub fn generate_openapi_spec_with_info(
        &self,
        title: &str,
        version: &str,
        description: Option<&str>,
        servers: Vec<OpenApiServer>,
    ) -> OpenApiSpec {
        let mut spec = self.generate_openapi_spec(title, version);

        if let Some(desc) = description {
            spec.info.description = Some(desc.to_string());
        }

        for server in servers {
            spec.add_server(server);
        }

        spec
    }

    /// Export OpenAPI spec as JSON string
    pub fn export_openapi_json(
        &self,
        title: &str,
        version: &str,
    ) -> Result<String, serde_json::Error> {
        let spec = self.generate_openapi_spec(title, version);
        serde_json::to_string_pretty(&spec)
    }
}

impl Default for MethodRegistry {
    fn default() -> Self {
        Self::empty()
    }
}

#[async_trait::async_trait]
impl MessageProcessor for MethodRegistry {
    async fn process_message(&self, message: Message) -> Option<Response> {
        match message {
            Message::Request(request) => {
                tracing::trace!(method = %request.method, correlation_id = ?request.correlation_id, "processing request");
                let response = self.call(&request.method, request.params, request.id).await;
                Some(response)
            }
            Message::Notification(notification) => {
                tracing::trace!(method = %notification.method, "processing notification");
                let _ = self
                    .call(&notification.method, notification.params, None)
                    .await;
                None
            }
            Message::Response(_) => None,
        }
    }

    async fn process_batch(&self, messages: Vec<Message>) -> Vec<Response> {
        let capabilities = self.get_capabilities();

        // Validate batch size
        if let Some(max_size) = capabilities.max_batch_size
            && messages.len() > max_size
        {
            tracing::warn!(
                batch_size = messages.len(),
                max_batch_size = max_size,
                "batch size limit exceeded"
            );
            return vec![crate::Response::error(
                crate::ErrorBuilder::new(
                    crate::error_codes::INVALID_REQUEST,
                    format!("Batch size {} exceeds maximum {}", messages.len(), max_size),
                )
                .build(),
                None,
            )];
        }

        tracing::debug!(batch_size = messages.len(), "processing batch");
        let mut results = Vec::new();
        for msg in messages {
            if let Some(response) = self.process_message(msg).await {
                results.push(response);
            }
        }
        results
    }

    fn get_capabilities(&self) -> ProcessorCapabilities {
        ProcessorCapabilities {
            supports_batch: true,
            supports_notifications: true,
            max_batch_size: Some(100),
            max_request_size: Some(1024 * 1024), // 1 MB
            request_timeout_secs: Some(30),
            supported_versions: vec!["2.0".to_string()],
        }
    }
}

#[async_trait::async_trait]
impl Handler for MethodRegistry {
    async fn handle_request(&self, request: Request) -> Response {
        self.call(&request.method, request.params, request.id).await
    }

    async fn handle_notification(&self, notification: Notification) {
        let _ = self
            .call(&notification.method, notification.params, None)
            .await;
    }

    fn supports_method(&self, method: &str) -> bool {
        self.has_method(method)
    }

    fn get_supported_methods(&self) -> Vec<String> {
        self.get_methods()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Test method implementation
    struct TestMethod {
        name: &'static str,
    }

    #[async_trait::async_trait]
    impl JsonRPCMethod for TestMethod {
        fn method_name(&self) -> &'static str {
            self.name
        }

        async fn call(
            &self,
            _params: Option<serde_json::Value>,
            id: Option<RequestId>,
        ) -> Response {
            ResponseBuilder::new()
                .success(json!({"method": self.name}))
                .id(id)
                .build()
        }
    }

    // Simple auth policy for testing
    struct TestAuthPolicy {
        allowed_methods: Vec<String>,
    }

    impl crate::auth::AuthPolicy for TestAuthPolicy {
        fn can_access(
            &self,
            method: &str,
            _params: Option<&serde_json::Value>,
            _ctx: &crate::auth::ConnectionContext,
        ) -> bool {
            self.allowed_methods.contains(&method.to_string())
        }

        fn unauthorized_error(&self, method: &str) -> Response {
            ResponseBuilder::new()
                .error(
                    ErrorBuilder::new(-32001, format!("Access denied for method '{}'", method))
                        .build(),
                )
                .build()
        }
    }

    #[tokio::test]
    async fn test_registry_without_auth() {
        let registry = MethodRegistry::new(vec![Box::new(TestMethod {
            name: "test_method",
        })]);

        let response = registry.call("test_method", None, Some(json!(1))).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_registry_with_auth_allowed() {
        let auth = TestAuthPolicy {
            allowed_methods: vec!["allowed_method".to_string()],
        };

        let registry = MethodRegistry::new(vec![Box::new(TestMethod {
            name: "allowed_method",
        })])
        .with_auth(auth);

        let response = registry.call("allowed_method", None, Some(json!(1))).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_registry_with_auth_denied() {
        let auth = TestAuthPolicy {
            allowed_methods: vec!["allowed_method".to_string()],
        };

        let registry = MethodRegistry::new(vec![Box::new(TestMethod {
            name: "blocked_method",
        })])
        .with_auth(auth);

        let response = registry.call("blocked_method", None, Some(json!(1))).await;
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, -32001);
        assert!(error.message.contains("Access denied"));
    }

    #[tokio::test]
    async fn test_registry_allow_all() {
        let registry = MethodRegistry::new(vec![Box::new(TestMethod { name: "any_method" })])
            .with_auth(crate::auth::AllowAll);

        let response = registry.call("any_method", None, Some(json!(1))).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_registry_deny_all() {
        let registry = MethodRegistry::new(vec![Box::new(TestMethod { name: "any_method" })])
            .with_auth(crate::auth::DenyAll);

        let response = registry.call("any_method", None, Some(json!(1))).await;
        assert!(response.result.is_none());
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_registry_empty() {
        let registry = MethodRegistry::empty();
        assert_eq!(registry.method_count(), 0);
    }

    #[tokio::test]
    async fn test_registry_default() {
        let registry = MethodRegistry::default();
        assert_eq!(registry.method_count(), 0);
    }

    #[tokio::test]
    async fn test_registry_add_method() {
        let registry = MethodRegistry::empty()
            .add_method(Box::new(TestMethod { name: "method1" }))
            .add_method(Box::new(TestMethod { name: "method2" }));

        assert_eq!(registry.method_count(), 2);
        assert!(registry.has_method("method1"));
        assert!(registry.has_method("method2"));
    }

    #[tokio::test]
    async fn test_registry_has_method() {
        let registry = MethodRegistry::new(vec![Box::new(TestMethod { name: "exists" })]);

        assert!(registry.has_method("exists"));
        assert!(!registry.has_method("not_exists"));
    }

    #[tokio::test]
    async fn test_registry_get_methods() {
        let registry = MethodRegistry::new(vec![
            Box::new(TestMethod { name: "method1" }),
            Box::new(TestMethod { name: "method2" }),
            Box::new(TestMethod { name: "method3" }),
        ]);

        let methods = registry.get_methods();
        assert_eq!(methods.len(), 3);
        assert!(methods.contains(&"method1".to_string()));
        assert!(methods.contains(&"method2".to_string()));
        assert!(methods.contains(&"method3".to_string()));
    }

    #[tokio::test]
    async fn test_registry_method_count() {
        let registry = MethodRegistry::new(vec![
            Box::new(TestMethod { name: "m1" }),
            Box::new(TestMethod { name: "m2" }),
        ]);

        assert_eq!(registry.method_count(), 2);
    }

    #[tokio::test]
    async fn test_registry_call_method_not_found() {
        let registry = MethodRegistry::new(vec![Box::new(TestMethod { name: "exists" })]);

        let response = registry.call("not_exists", None, Some(json!(1))).await;
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, error_codes::METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_registry_call_with_params() {
        let registry = MethodRegistry::new(vec![Box::new(TestMethod { name: "test" })]);

        let params = json!({"key": "value"});
        let response = registry.call("test", Some(params), Some(json!(1))).await;
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_registry_call_with_context() {
        let registry = MethodRegistry::new(vec![Box::new(TestMethod { name: "test" })]);

        let ctx = crate::auth::ConnectionContext::default();
        let response = registry
            .call_with_context("test", None, Some(json!(1)), &ctx)
            .await;
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_registry_call_with_context_auth_denied() {
        let auth = TestAuthPolicy {
            allowed_methods: vec!["allowed".to_string()],
        };

        let registry =
            MethodRegistry::new(vec![Box::new(TestMethod { name: "blocked" })]).with_auth(auth);

        let ctx = crate::auth::ConnectionContext::default();
        let response = registry
            .call_with_context("blocked", None, Some(json!(1)), &ctx)
            .await;
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_registry_generate_openapi_spec() {
        let registry = MethodRegistry::new(vec![
            Box::new(TestMethod { name: "method1" }),
            Box::new(TestMethod { name: "method2" }),
        ]);

        let spec = registry.generate_openapi_spec("Test API", "1.0.0");
        assert_eq!(spec.info.title, "Test API");
        assert_eq!(spec.info.version, "1.0.0");
        assert_eq!(spec.methods.len(), 2);
        assert!(spec.methods.contains_key("method1"));
        assert!(spec.methods.contains_key("method2"));
    }

    #[tokio::test]
    async fn test_registry_generate_openapi_spec_with_info() {
        let registry = MethodRegistry::new(vec![Box::new(TestMethod { name: "test" })]);

        let servers = vec![OpenApiServer::new("http://localhost:8080")];

        let spec = registry.generate_openapi_spec_with_info(
            "API",
            "2.0.0",
            Some("Test description"),
            servers,
        );

        assert_eq!(spec.info.title, "API");
        assert_eq!(spec.info.version, "2.0.0");
        assert_eq!(spec.info.description, Some("Test description".to_string()));
        assert_eq!(spec.servers.len(), 1);
        assert_eq!(spec.servers[0].url, "http://localhost:8080");
    }

    #[tokio::test]
    async fn test_registry_export_openapi_json() {
        let registry = MethodRegistry::new(vec![Box::new(TestMethod { name: "test" })]);

        let json_str = registry.export_openapi_json("API", "1.0").unwrap();
        assert!(json_str.contains("\"title\": \"API\""));
        assert!(json_str.contains("\"version\": \"1.0\""));
        assert!(json_str.contains("\"openapi\": \"3.0.3\""));
    }

    #[tokio::test]
    async fn test_registry_message_processor_request() {
        let registry = MethodRegistry::new(vec![Box::new(TestMethod { name: "test" })]);

        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "test".to_string(),
            params: None,
            id: Some(json!(1)),
            correlation_id: None,
        };

        let response = registry.process_message(Message::Request(request)).await;
        assert!(response.is_some());
        assert!(response.unwrap().result.is_some());
    }

    #[tokio::test]
    async fn test_registry_message_processor_notification() {
        let registry = MethodRegistry::new(vec![Box::new(TestMethod { name: "test" })]);

        let notification = Notification {
            jsonrpc: "2.0".to_string(),
            method: "test".to_string(),
            params: None,
        };

        let response = registry
            .process_message(Message::Notification(notification))
            .await;
        assert!(response.is_none());
    }

    #[tokio::test]
    async fn test_registry_message_processor_response() {
        let registry = MethodRegistry::new(vec![]);

        let response_msg = Response {
            jsonrpc: "2.0".to_string(),
            result: Some(json!(42)),
            error: None,
            id: Some(json!(1)),
            correlation_id: None,
        };

        let response = registry
            .process_message(Message::Response(response_msg))
            .await;
        assert!(response.is_none());
    }

    #[tokio::test]
    async fn test_registry_process_batch() {
        let registry = MethodRegistry::new(vec![Box::new(TestMethod { name: "test" })]);

        let messages = vec![
            Message::Request(Request {
                jsonrpc: "2.0".to_string(),
                method: "test".to_string(),
                params: None,
                id: Some(json!(1)),
                correlation_id: None,
            }),
            Message::Request(Request {
                jsonrpc: "2.0".to_string(),
                method: "test".to_string(),
                params: None,
                id: Some(json!(2)),
                correlation_id: None,
            }),
        ];

        let responses = registry.process_batch(messages).await;
        assert_eq!(responses.len(), 2);
    }

    #[test]
    fn test_register_methods_macro() {
        let methods = register_methods![TestMethod { name: "m1" }, TestMethod { name: "m2" },];
        assert_eq!(methods.len(), 2);
    }
}
