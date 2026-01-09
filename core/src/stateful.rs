//! # Stateful JSON-RPC Handlers
//!
//! Stateful JSON-RPC handlers with shared context support.
//!
//! This module extends ash-rpc-core with stateful method handlers that can access
//! shared application state through a service context.
//!
//! ## Features
//!
//! - **Shared context** - Pass application state to method handlers
//! - **Error handling** - Proper error propagation through the context system
//! - **Method registry** - Organize stateful methods in a registry
//! - **Type safety** - Generic over context types for compile-time guarantees

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
    
    /// Get method documentation (return empty string for now)
    fn documentation(&self) -> String {
        String::new()
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
                return handler.call(context, params, id).await;
            }
        }
        
        // Method not found
        Ok(ResponseBuilder::new()
            .error(ErrorBuilder::new(error_codes::METHOD_NOT_FOUND, "Method not found").build())
            .id(id)
            .build())
    }
    
    /// Get the method name from a trait implementation
    fn get_method_name<M: StatefulJsonRPCMethod<C>>() -> &'static str {
        M::METHOD_NAME
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
                match self.handler.handle_request(&self.context, request).await {
                    Ok(response) => Some(response),
                    Err(_) => Some(
                        ResponseBuilder::new()
                            .error(
                                ErrorBuilder::new(
                                    error_codes::INTERNAL_ERROR,
                                    "Internal server error",
                                )
                                .build(),
                            )
                            .id(None)
                            .build(),
                    ),
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
