//! Core traits for JSON-RPC handlers and processors.

use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
