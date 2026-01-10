//! # ash-rpc-core
//!
//! A comprehensive JSON-RPC 2.0 implementation with transport support.
//!
//! ## Features
//!
//! - **Complete JSON-RPC 2.0 support** - Request, response, notification, and batch handling
//! - **Multiple transports** - TCP, TCP streaming, WebSocket, HTTP via Axum, and Tower middleware
//! - **Stateful handlers** - Context-aware method handlers with shared application state
//! - **Type-safe builders** - Fluent API for constructing requests and responses
//! - **Method registry** - Organize and dispatch JSON-RPC methods
//! - **Auto-documentation** - Generate OpenAPI/Swagger specs from method definitions
//! - **Code generation** - CLI tool for generating boilerplate implementations
//! - **Macro support** - Convenient macros for common response patterns
//!
//! ## Quick Start
//!
//! ```rust
//! use ash_rpc_core::*;
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
//! // Create a method registry
//! let registry = MethodRegistry::new(register_methods![PingMethod]);
//! ```

// Module declarations
pub mod auth;
pub mod builders;
pub mod macros;
pub mod registry;
pub mod sanitization;
pub mod traits;
pub mod transport;
pub mod types;

#[cfg(feature = "stateful")]
pub mod stateful;

// Re-export async_trait for users implementing traits
pub use async_trait::async_trait;

// Re-export tokio for tcp-stream feature
#[cfg(feature = "tcp-stream")]
pub use tokio;

// Re-export all core types
pub use types::*;

// Re-export all builders
pub use builders::*;

// Re-export all traits
pub use traits::*;

// Re-export registry
pub use registry::*;

// Re-export transport functionality when needed
// pub use transport::*;

// Re-export stateful module when stateful feature is enabled
#[cfg(feature = "stateful")]
pub use stateful::*;
