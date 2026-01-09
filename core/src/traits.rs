//! Core traits for JSON-RPC handlers and processors.

use crate::types::*;
use std::future::Future;
use std::pin::Pin;

/// Async trait for individual JSON-RPC method implementations
pub trait JsonRPCMethod: Send + Sync {
    /// Get the method name that this implementation handles
    fn method_name(&self) -> &'static str;
    
    /// Execute the JSON-RPC method asynchronously
    fn call<'a>(
        &'a self,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>>;
    
    /// Get method documentation (return empty string for now)
    fn documentation(&self) -> String {
        String::new()
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

    /// Get method information for documentation
    fn get_method_info(&self, method: &str) -> Option<MethodInfo> {
        let _ = method;
        None
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

/// Method information for documentation and introspection
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub description: Option<String>,
    pub params_schema: Option<serde_json::Value>,
    pub result_schema: Option<serde_json::Value>,
}

impl MethodInfo {
    /// Create new method info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            params_schema: None,
            result_schema: None,
        }
    }

    /// Add method description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add parameter schema
    pub fn with_params_schema(mut self, schema: serde_json::Value) -> Self {
        self.params_schema = Some(schema);
        self
    }

    /// Add result schema
    pub fn with_result_schema(mut self, schema: serde_json::Value) -> Self {
        self.result_schema = Some(schema);
        self
    }
}

#[derive(Debug, Clone)]
pub struct ProcessorCapabilities {
    pub supports_batch: bool,
    pub supports_notifications: bool,
    pub max_batch_size: Option<usize>,
    pub supported_versions: Vec<String>,
}

impl Default for ProcessorCapabilities {
    fn default() -> Self {
        Self {
            supports_batch: true,
            supports_notifications: true,
            max_batch_size: None,
            supported_versions: vec!["2.0".to_string()],
        }
    }
}
