//! # Stateful JSON-RPC Handlers
//!
//! Stateful JSON-RPC handlers with shared context support.
//!
//! This module extends ash-rpc-core with stateful method handlers that can access
//! shared application state through a service context.
//!

use crate::{
    ErrorBuilder, Message, MessageProcessor, Request, Response, ResponseBuilder, error_codes,
};
use std::sync::Arc;

/// Trait for service context shared across stateful handlers
pub trait ServiceContext: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;
}

/// Async trait for stateful JSON-RPC method implementations with context
#[async_trait::async_trait]
pub trait StatefulJsonRPCMethod<C: ServiceContext>: Send + Sync {
    /// Get the method name for runtime dispatch
    fn method_name(&self) -> &'static str;
    
    /// Execute the JSON-RPC method asynchronously with context
    async fn call(
        &self,
        context: &C,
        params: Option<serde_json::Value>,
        id: Option<crate::RequestId>,
    ) -> Result<Response, C::Error>;
    
    /// Get OpenAPI components for this method
    fn openapi_components(&self) -> crate::traits::OpenApiMethodSpec {
        crate::traits::OpenApiMethodSpec::new(self.method_name())
    }
}

/// Trait for stateful JSON-RPC handlers
#[async_trait::async_trait]
pub trait StatefulHandler<C: ServiceContext>: Send + Sync {
    /// Handle a JSON-RPC request with access to shared context
    async fn handle_request(&self, context: &C, request: Request) -> Result<Response, C::Error>;

    /// Handle a JSON-RPC notification with access to shared context
    async fn handle_notification(
        &self,
        context: &C,
        notification: crate::Notification,
    ) -> Result<(), C::Error> {
        let _ = context;
        let _ = notification;
        Ok(())
    }
}

/// Registry for organizing stateful JSON-RPC methods
pub struct StatefulMethodRegistry<C: ServiceContext> {
    methods: Vec<Box<dyn StatefulJsonRPCMethod<C>>>,
}

impl<C: ServiceContext> StatefulMethodRegistry<C> {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            methods: Vec::new(),
        }
    }

    /// Register a method handler
    pub fn register<M>(mut self, method: M) -> Self
    where
        M: StatefulJsonRPCMethod<C> + 'static,
    {
        tracing::trace!("registering stateful method");
        self.methods.push(Box::new(method));
        self
    }

    /// Call a registered method with context
    pub async fn call(
        &self,
        context: &C,
        method: &str,
        params: Option<serde_json::Value>,
        id: Option<crate::RequestId>,
    ) -> Result<Response, C::Error> {
        // Generate match statement for all registered methods
        for handler in &self.methods {
            if handler.method_name() == method {
                tracing::debug!(method = %method, "calling stateful method");
                return handler.call(context, params, id).await;
            }
        }
        
        tracing::warn!(method = %method, "stateful method not found");
        // Method not found
        Ok(ResponseBuilder::new()
            .error(ErrorBuilder::new(error_codes::METHOD_NOT_FOUND, "Method not found").build())
            .id(id)
            .build())
    }
}

impl<C: ServiceContext> Default for StatefulMethodRegistry<C> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl<C: ServiceContext> StatefulHandler<C> for StatefulMethodRegistry<C> {
    async fn handle_request(&self, context: &C, request: Request) -> Result<Response, C::Error> {
        self.call(context, &request.method, request.params, request.id).await
    }

    async fn handle_notification(
        &self,
        context: &C,
        notification: crate::Notification,
    ) -> Result<(), C::Error> {
        let _ = self.call(context, &notification.method, notification.params, None).await?;
        Ok(())
    }
}

/// Stateful message processor that wraps a context and handler
pub struct StatefulProcessor<C: ServiceContext> {
    context: Arc<C>,
    handler: Arc<dyn StatefulHandler<C>>,
}

impl<C: ServiceContext> StatefulProcessor<C> {
    /// Create a new stateful processor with context and handler
    pub fn new<H>(context: C, handler: H) -> Self
    where
        H: StatefulHandler<C> + 'static,
    {
        Self {
            context: Arc::new(context),
            handler: Arc::new(handler),
        }
    }

    /// Create a builder for configuring the processor
    pub fn builder(context: C) -> StatefulProcessorBuilder<C> {
        StatefulProcessorBuilder::new(context)
    }
}

#[async_trait::async_trait]
impl<C: ServiceContext> MessageProcessor for StatefulProcessor<C> {
    async fn process_message(&self, message: Message) -> Option<Response> {
        match message {
            Message::Request(request) => {
                let request_id = request.id.clone();
                let correlation_id = request.correlation_id.clone();
                
                match self.handler.handle_request(&self.context, request).await {
                    Ok(response) => Some(response),
                    Err(error) => {
                        // Log the actual error with correlation tracking
                        tracing::error!(
                            error = %error,
                            request_id = ?request_id,
                            correlation_id = ?correlation_id,
                            "stateful handler error"
                        );
                        
                        // Return generic error that preserves request ID
                        // Users can customize error handling by implementing their own MessageProcessor
                        let generic_error = crate::Error::from_error_logged(&error as &dyn std::error::Error);
                        
                        Some(
                            ResponseBuilder::new()
                                .error(generic_error)
                                .id(request_id)  // Preserve request ID for correlation
                                .correlation_id(correlation_id)  // Preserve correlation ID
                                .build(),
                        )
                    }
                }
            }
            Message::Notification(notification) => {
                let _ = self
                    .handler
                    .handle_notification(&self.context, notification).await;
                None
            }
            Message::Response(_) => None,
        }
    }
}

/// Builder for creating stateful processors
pub struct StatefulProcessorBuilder<C: ServiceContext> {
    context: C,
    handler: Option<Arc<dyn StatefulHandler<C>>>,
}

impl<C: ServiceContext> StatefulProcessorBuilder<C> {
    /// Create a new builder with the given context
    pub fn new(context: C) -> Self {
        Self {
            context,
            handler: None,
        }
    }

    /// Set the handler for processing requests
    pub fn handler<H>(mut self, handler: H) -> Self
    where
        H: StatefulHandler<C> + 'static,
    {
        self.handler = Some(Arc::new(handler));
        self
    }

    /// Set a method registry as the handler
    pub fn registry(mut self, registry: StatefulMethodRegistry<C>) -> Self {
        self.handler = Some(Arc::new(registry));
        self
    }

    /// Build the stateful processor
    pub fn build(self) -> Result<StatefulProcessor<C>, Box<dyn std::error::Error>> {
        let handler = self.handler.ok_or("Handler not set")?;
        Ok(StatefulProcessor {
            context: Arc::new(self.context),
            handler,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RequestBuilder, Notification};
    use std::sync::atomic::{AtomicU32, Ordering};

    // Test context implementation
    #[derive(Debug)]
    struct TestError(String);
    
    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    
    impl std::error::Error for TestError {}

    struct TestContext {
        counter: AtomicU32,
    }

    impl ServiceContext for TestContext {
        type Error = TestError;
    }

    impl TestContext {
        fn new() -> Self {
            Self {
                counter: AtomicU32::new(0),
            }
        }

        fn increment(&self) -> u32 {
            self.counter.fetch_add(1, Ordering::SeqCst) + 1
        }

        fn get_count(&self) -> u32 {
            self.counter.load(Ordering::SeqCst)
        }
    }

    // Test method implementation
    struct IncrementMethod;

    #[async_trait::async_trait]
    impl StatefulJsonRPCMethod<TestContext> for IncrementMethod {
        fn method_name(&self) -> &'static str {
            "increment"
        }

        async fn call(
            &self,
            context: &TestContext,
            _params: Option<serde_json::Value>,
            id: Option<crate::RequestId>,
        ) -> Result<Response, TestError> {
            let count = context.increment();
            Ok(ResponseBuilder::new()
                .success(serde_json::json!({"count": count}))
                .id(id)
                .build())
        }
    }

    // Failing method for error tests
    struct FailingMethod;

    #[async_trait::async_trait]
    impl StatefulJsonRPCMethod<TestContext> for FailingMethod {
        fn method_name(&self) -> &'static str {
            "fail"
        }

        async fn call(
            &self,
            _context: &TestContext,
            _params: Option<serde_json::Value>,
            _id: Option<crate::RequestId>,
        ) -> Result<Response, TestError> {
            Err(TestError("intentional failure".to_string()))
        }
    }

    #[tokio::test]
    async fn test_stateful_registry_register_and_call() {
        let context = TestContext::new();
        let registry = StatefulMethodRegistry::new().register(IncrementMethod);

        let result = registry
            .call(&context, "increment", None, Some(serde_json::json!(1)))
            .await
            .unwrap();

        assert!(result.result.is_some());
        assert_eq!(context.get_count(), 1);
    }

    #[tokio::test]
    async fn test_stateful_registry_method_not_found() {
        let context = TestContext::new();
        let registry = StatefulMethodRegistry::<TestContext>::new();

        let result = registry
            .call(&context, "unknown", None, Some(serde_json::json!(1)))
            .await
            .unwrap();

        assert!(result.error.is_some());
        let error = result.error.unwrap();
        assert_eq!(error.code, error_codes::METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_stateful_registry_multiple_methods() {
        let context = TestContext::new();
        let registry = StatefulMethodRegistry::new()
            .register(IncrementMethod)
            .register(FailingMethod);

        // Call increment twice
        let _ = registry.call(&context, "increment", None, Some(serde_json::json!(1))).await;
        let _ = registry.call(&context, "increment", None, Some(serde_json::json!(2))).await;
        assert_eq!(context.get_count(), 2);

        // Call failing method
        let result = registry
            .call(&context, "fail", None, Some(serde_json::json!(3)))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stateful_handler_request() {
        let context = TestContext::new();
        let registry = StatefulMethodRegistry::new().register(IncrementMethod);

        let request = RequestBuilder::new("increment")
            .id(serde_json::json!(1))
            .build();

        let result = registry.handle_request(&context, request).await.unwrap();
        assert!(result.result.is_some());
    }

    #[tokio::test]
    async fn test_stateful_handler_notification() {
        let context = TestContext::new();
        let registry = StatefulMethodRegistry::new().register(IncrementMethod);

        let notification = Notification {
            jsonrpc: "2.0".to_string(),
            method: "increment".to_string(),
            params: None,
        };

        let result = registry.handle_notification(&context, notification).await;
        assert!(result.is_ok());
        assert_eq!(context.get_count(), 1);
    }

    #[tokio::test]
    async fn test_stateful_processor_request() {
        let context = TestContext::new();
        let registry = StatefulMethodRegistry::new().register(IncrementMethod);
        let processor = StatefulProcessor::new(context, registry);

        let request = RequestBuilder::new("increment")
            .id(serde_json::json!(1))
            .build();

        let response = processor.process_message(Message::Request(request)).await;
        assert!(response.is_some());
        let response = response.unwrap();
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_stateful_processor_notification() {
        let context = TestContext::new();
        let registry = StatefulMethodRegistry::new().register(IncrementMethod);
        let processor = StatefulProcessor::new(context, registry);

        let notification = Notification {
            jsonrpc: "2.0".to_string(),
            method: "increment".to_string(),
            params: None,
        };

        let response = processor.process_message(Message::Notification(notification)).await;
        assert!(response.is_none());
    }

    #[tokio::test]
    async fn test_stateful_processor_error_handling() {
        let context = TestContext::new();
        let registry = StatefulMethodRegistry::new().register(FailingMethod);
        let processor = StatefulProcessor::new(context, registry);

        let request = RequestBuilder::new("fail")
            .id(serde_json::json!(1))
            .build();

        let response = processor.process_message(Message::Request(request)).await;
        assert!(response.is_some());
        let response = response.unwrap();
        assert!(response.error.is_some());
        assert_eq!(response.id, Some(serde_json::json!(1)));
    }

    #[tokio::test]
    async fn test_stateful_processor_preserves_correlation_id() {
        let context = TestContext::new();
        let registry = StatefulMethodRegistry::new().register(FailingMethod);
        let processor = StatefulProcessor::new(context, registry);

        let correlation_id = uuid::Uuid::new_v4().to_string();
        let request = RequestBuilder::new("fail")
            .id(serde_json::json!(1))
            .correlation_id(correlation_id.clone())
            .build();

        let response = processor.process_message(Message::Request(request)).await.unwrap();
        assert_eq!(response.correlation_id, Some(correlation_id));
    }

    #[tokio::test]
    async fn test_stateful_processor_builder() {
        let context = TestContext::new();
        let registry = StatefulMethodRegistry::new().register(IncrementMethod);
        
        let processor = StatefulProcessor::builder(context)
            .registry(registry)
            .build()
            .unwrap();

        let request = RequestBuilder::new("increment")
            .id(serde_json::json!(1))
            .build();

        let response = processor.process_message(Message::Request(request)).await;
        assert!(response.is_some());
    }

    #[tokio::test]
    async fn test_stateful_processor_builder_no_handler() {
        let context = TestContext::new();
        let result = StatefulProcessor::builder(context).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_stateful_method_openapi_components() {
        let method = IncrementMethod;
        let spec = method.openapi_components();
        assert_eq!(spec.method_name, "increment");
    }

    #[test]
    fn test_stateful_registry_default() {
        let registry = StatefulMethodRegistry::<TestContext>::default();
        assert_eq!(registry.methods.len(), 0);
    }
}
