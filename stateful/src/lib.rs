//! # ash-rpc-stateful
//! 
//! Stateful JSON-RPC handlers with shared context support.
//! 
//! This crate extends ash-rpc-core with stateful method handlers that can access
//! shared application state through a service context.
//! 
//! ## Features
//! 
//! - **Shared context** - Pass application state to method handlers
//! - **Error handling** - Proper error propagation through the context system
//! - **Method registry** - Organize stateful methods in a registry
//! - **Type safety** - Generic over context types for compile-time guarantees

use ash_rpc_core::{Request, Response, Message, MessageProcessor};
use std::sync::Arc;

/// Trait for service context shared across stateful handlers
pub trait ServiceContext: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;
}

/// Trait for stateful JSON-RPC handlers
pub trait StatefulHandler<C: ServiceContext>: Send + Sync {
    /// Handle a JSON-RPC request with access to shared context
    fn handle_request(&self, context: &C, request: Request) -> Result<Response, C::Error>;
    
    /// Handle a JSON-RPC notification with access to shared context
    fn handle_notification(&self, context: &C, notification: ash_rpc_core::Notification) -> Result<(), C::Error> {
        let _ = context;
        let _ = notification;
        Ok(())
    }
}

/// Trait for stateful method handlers
pub trait StatefulMethodHandler<C: ServiceContext>: Send + Sync {
    /// Call the method handler with context and parameters
    fn call(&self, context: &C, params: Option<serde_json::Value>, id: Option<ash_rpc_core::RequestId>) -> Result<Response, C::Error>;
}

impl<C, F> StatefulMethodHandler<C> for F
where
    C: ServiceContext,
    F: Fn(&C, Option<serde_json::Value>, Option<ash_rpc_core::RequestId>) -> Result<Response, C::Error> + Send + Sync,
{
    fn call(&self, context: &C, params: Option<serde_json::Value>, id: Option<ash_rpc_core::RequestId>) -> Result<Response, C::Error> {
        self(context, params, id)
    }
}

/// Registry for organizing stateful JSON-RPC methods
pub struct StatefulMethodRegistry<C: ServiceContext> {
    methods: std::collections::HashMap<String, Box<dyn StatefulMethodHandler<C>>>,
}

impl<C: ServiceContext> StatefulMethodRegistry<C> {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            methods: std::collections::HashMap::new(),
        }
    }

    /// Register a method handler
    pub fn register<H>(mut self, method: impl Into<String>, handler: H) -> Self
    where
        H: StatefulMethodHandler<C> + 'static,
    {
        self.methods.insert(method.into(), Box::new(handler));
        self
    }

    /// Register a method using a closure
    pub fn register_fn<F>(mut self, method: impl Into<String>, handler: F) -> Self
    where
        F: Fn(&C, Option<serde_json::Value>, Option<ash_rpc_core::RequestId>) -> Result<Response, C::Error> + Send + Sync + 'static,
    {
        self.methods.insert(method.into(), Box::new(handler));
        self
    }

    /// Call a registered method with context
    pub fn call(&self, context: &C, method: &str, params: Option<serde_json::Value>, id: Option<ash_rpc_core::RequestId>) -> Result<Response, C::Error> {
        if let Some(handler) = self.methods.get(method) {
            handler.call(context, params, id)
        } else {
            Ok(ash_rpc_core::ResponseBuilder::new()
                .error(ash_rpc_core::ErrorBuilder::new(
                    ash_rpc_core::error_codes::METHOD_NOT_FOUND,
                    "Method not found"
                ).build())
                .id(id)
                .build())
        }
    }
}

impl<C: ServiceContext> Default for StatefulMethodRegistry<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: ServiceContext> StatefulHandler<C> for StatefulMethodRegistry<C> {
    fn handle_request(&self, context: &C, request: Request) -> Result<Response, C::Error> {
        self.call(context, &request.method, request.params, request.id)
    }

    fn handle_notification(&self, context: &C, notification: ash_rpc_core::Notification) -> Result<(), C::Error> {
        let _ = self.call(context, &notification.method, notification.params, None)?;
        Ok(())
    }
}

pub struct StatefulProcessor<C: ServiceContext> {
    context: Arc<C>,
    handler: Arc<dyn StatefulHandler<C>>,
}

impl<C: ServiceContext> StatefulProcessor<C> {
    pub fn new<H>(context: C, handler: H) -> Self
    where
        H: StatefulHandler<C> + 'static,
    {
        Self {
            context: Arc::new(context),
            handler: Arc::new(handler),
        }
    }

    pub fn builder(context: C) -> StatefulProcessorBuilder<C> {
        StatefulProcessorBuilder::new(context)
    }
}

impl<C: ServiceContext> MessageProcessor for StatefulProcessor<C> {
    fn process_message(&self, message: Message) -> Option<Response> {
        match message {
            Message::Request(request) => {
                match self.handler.handle_request(&self.context, request) {
                    Ok(response) => Some(response),
                    Err(_) => {
                        Some(ash_rpc_core::ResponseBuilder::new()
                            .error(ash_rpc_core::ErrorBuilder::new(
                                ash_rpc_core::error_codes::INTERNAL_ERROR,
                                "Internal server error"
                            ).build())
                            .id(None)
                            .build())
                    }
                }
            }
            Message::Notification(notification) => {
                let _ = self.handler.handle_notification(&self.context, notification);
                None
            }
            Message::Response(_) => None,
        }
    }
}

pub struct StatefulProcessorBuilder<C: ServiceContext> {
    context: C,
    handler: Option<Arc<dyn StatefulHandler<C>>>,
}

impl<C: ServiceContext> StatefulProcessorBuilder<C> {
    pub fn new(context: C) -> Self {
        Self {
            context,
            handler: None,
        }
    }

    pub fn handler<H>(mut self, handler: H) -> Self
    where
        H: StatefulHandler<C> + 'static,
    {
        self.handler = Some(Arc::new(handler));
        self
    }

    pub fn registry(mut self, registry: StatefulMethodRegistry<C>) -> Self {
        self.handler = Some(Arc::new(registry));
        self
    }

    pub fn build(self) -> Result<StatefulProcessor<C>, Box<dyn std::error::Error>> {
        let handler = self.handler.ok_or("Handler not set")?;
        Ok(StatefulProcessor {
            context: Arc::new(self.context),
            handler,
        })
    }
}
