//! Method registry for organizing and dispatching JSON-RPC methods.
//! 
//! ## Usage
//! 
//! ### Basic Usage (Runtime Dispatch)
//! Create method implementations using the `JsonRPCMethod` trait:
//! 
//! ```rust
//! use ash_rpc_core::*;
//! 
//! struct PingMethod;
//! 
//! impl JsonRPCMethod for PingMethod {
//!     fn method_name(&self) -> &'static str { "ping" }
//!     
//!     fn call<'a>(
//!         &'a self,
//!         _params: Option<serde_json::Value>,
//!         id: Option<RequestId>,
//!     ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
//!         Box::pin(async move {
//!             rpc_success!("pong", id)
//!         })
//!     }
//! }
//! 
//! let registry = MethodRegistry::new(register_methods![PingMethod]);
//! ```
//!
//! ### Optimized Usage (Compile-time Dispatch)
//! For better performance, use the dispatch_call! macro:
//!
//! ```rust
//! // In your handler function:
//! async fn handle_call(method_name: &str, params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
//!     dispatch_call!(method_name, params, id => PingMethod, EchoMethod, CalculatorMethod)
//! }
//! ```

use crate::builders::*;
use crate::traits::*;
use crate::types::*;

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

/// Registry for organizing and dispatching async JSON-RPC methods
pub struct MethodRegistry {
    methods: Vec<Box<dyn JsonRPCMethod>>,
}

impl MethodRegistry {
    /// Create a new method registry with the given method implementations
    pub fn new(methods: Vec<Box<dyn JsonRPCMethod>>) -> Self {
        Self { methods }
    }

    /// Create an empty registry
    pub fn empty() -> Self {
        Self {
            methods: Vec::new(),
        }
    }

    /// Add a method implementation to the registry
    pub fn add_method(mut self, method: Box<dyn JsonRPCMethod>) -> Self {
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
        // Fallback to runtime dispatch if compile-time dispatch is not used
        for method in &self.methods {
            if method.method_name() == method_name {
                return method.call(params, id).await;
            }
        }
        
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
        self.methods.iter().map(|m| m.method_name().to_string()).collect()
    }

    /// Get the number of registered methods
    pub fn method_count(&self) -> usize {
        self.methods.len()
    }

    /// Generate documentation for all registered methods
    pub fn render_docs(&self) -> serde_json::Value {
        let mut docs = serde_json::Map::new();
        
        for method in &self.methods {
            let method_name = method.method_name();
            let documentation = method.documentation();
            
            docs.insert(
                method_name.to_string(),
                serde_json::json!({
                    "name": method_name,
                    "description": documentation
                })
            );
        }
        
        serde_json::Value::Object(docs)
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
                let response = self.call(&request.method, request.params, request.id).await;
                Some(response)
            }
            Message::Notification(notification) => {
                let _ = self.call(&notification.method, notification.params, None).await;
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

#[async_trait::async_trait]
impl Handler for MethodRegistry {
    async fn handle_request(&self, request: Request) -> Response {
        self.call(&request.method, request.params, request.id).await
    }

    async fn handle_notification(&self, notification: Notification) {
        let _ = self.call(&notification.method, notification.params, None).await;
    }

    fn supports_method(&self, method: &str) -> bool {
        self.has_method(method)
    }

    fn get_supported_methods(&self) -> Vec<String> {
        self.get_methods()
    }
}
