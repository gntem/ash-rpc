//! Method registry for organizing and dispatching JSON-RPC methods.

use crate::builders::*;
use crate::traits::*;
use crate::types::*;
use crate::utils;
use std::collections::HashMap;

/// Function signature for method handlers
pub type MethodHandler =
    Box<dyn Fn(Option<serde_json::Value>, Option<RequestId>) -> Response + Send + Sync>;

/// Registry for organizing and dispatching JSON-RPC methods
pub struct MethodRegistry {
    methods: HashMap<String, MethodHandler>,
    method_info: HashMap<String, MethodInfo>,
    cached_docs: Option<String>,
}

impl MethodRegistry {
    /// Create a new empty method registry
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
            method_info: HashMap::new(),
            cached_docs: None,
        }
    }

    /// Register a method with a handler function
    pub fn register<F>(mut self, method: impl Into<String>, handler: F) -> Self
    where
        F: Fn(Option<serde_json::Value>, Option<RequestId>) -> Response + Send + Sync + 'static,
    {
        let method_name = method.into();
        self.method_info
            .insert(method_name.clone(), MethodInfo::new(method_name.clone()));
        self.methods.insert(method_name, Box::new(handler));
        self.cached_docs = None;
        self
    }

    /// Register a method with detailed information and handler
    pub fn register_with_info<F>(
        mut self,
        method: impl Into<String>,
        info: MethodInfo,
        handler: F,
    ) -> Self
    where
        F: Fn(Option<serde_json::Value>, Option<RequestId>) -> Response + Send + Sync + 'static,
    {
        let method_name = method.into();
        self.method_info.insert(method_name.clone(), info);
        self.methods.insert(method_name, Box::new(handler));
        self.cached_docs = None;
        self
    }

    /// Call a registered method
    pub fn call(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Response {
        if let Some(handler) = self.methods.get(method) {
            handler(params, id)
        } else {
            ResponseBuilder::new()
                .error(ErrorBuilder::new(error_codes::METHOD_NOT_FOUND, "Method not found").build())
                .id(id)
                .build()
        }
    }

    /// Check if a method is registered
    pub fn has_method(&self, method: &str) -> bool {
        self.methods.contains_key(method)
    }

    /// Get list of all registered methods
    pub fn get_methods(&self) -> Vec<String> {
        self.methods.keys().cloned().collect()
    }

    /// Get the number of registered methods
    pub fn method_count(&self) -> usize {
        self.methods.len()
    }

    /// Remove a method from the registry
    pub fn remove_method(&mut self, method: &str) -> bool {
        self.method_info.remove(method);
        self.cached_docs = None;
        self.methods.remove(method).is_some()
    }

    /// Clear all methods from the registry
    pub fn clear(&mut self) {
        self.methods.clear();
        self.method_info.clear();
        self.cached_docs = None;
    }

    /// Generate a Swagger/OpenAPI JSON object describing all registered methods
    /// Results are cached until the registry is modified
    pub fn render_docs(&mut self) -> serde_json::Value {
        if self.cached_docs.is_none() {
            let docs_json = utils::render_docs(&self.method_info);
            self.cached_docs = Some(serde_json::to_string(&docs_json).unwrap_or_default());
        }

        // Parse the cached string back to Value
        self.cached_docs
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    }
}

impl Default for MethodRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageProcessor for MethodRegistry {
    fn process_message(&self, message: Message) -> Option<Response> {
        match message {
            Message::Request(request) => {
                let response = self.call(&request.method, request.params, request.id);
                Some(response)
            }
            Message::Notification(notification) => {
                let _ = self.call(&notification.method, notification.params, None);
                None
            }
            Message::Response(_) => None,
        }
    }

    fn get_capabilities(&self) -> ProcessorCapabilities {
        ProcessorCapabilities {
            supports_batch: true,
            supports_notifications: true,
            max_batch_size: Some(100),
            supported_versions: vec!["2.0".to_string()],
        }
    }
}

impl Handler for MethodRegistry {
    fn handle_request(&self, request: Request) -> Response {
        self.call(&request.method, request.params, request.id)
    }

    fn handle_notification(&self, notification: Notification) {
        let _ = self.call(&notification.method, notification.params, None);
    }

    fn supports_method(&self, method: &str) -> bool {
        self.has_method(method)
    }

    fn get_supported_methods(&self) -> Vec<String> {
        self.get_methods()
    }
}
