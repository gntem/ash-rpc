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
//! ```rust,no_run
//! use ash_rpc_core::*;
//! use serde_json::Value;
//!
//! // Create a method registry
//! let registry = MethodRegistry::new()
//!     .register("ping", |_params, id| {
//!         rpc_success!("pong", id)
//!     })
//!     .register("add", |params, id| {
//!         if let Some(params) = params {
//!             let nums: Vec<i32> = serde_json::from_value(params).unwrap();
//!             if nums.len() == 2 {
//!                 rpc_success!(nums[0] + nums[1], id)
//!             } else {
//!                 rpc_error!(error_codes::INVALID_PARAMS, "Expected 2 numbers", id)
//!             }
//!         } else {
//!             rpc_error!(error_codes::INVALID_PARAMS, "Parameters required", id)
//!         }
//!     });
//!
//! // Call a method
//! let response = registry.call("ping", None, Some(Value::Number(serde_json::Number::from(1))));
//! ```

// Module declarations
pub mod builders;
pub mod macros;
pub mod registry;
pub mod traits;
pub mod transport;
pub mod types;

#[cfg(feature = "tower")]
pub mod middleware;

#[cfg(feature = "stateful")]
pub mod stateful;

// Re-export tokio for tcp-stream feature
#[cfg(feature = "tcp-stream")]
pub use tokio;

// Re-export tower for tower feature
#[cfg(feature = "tower")]
pub use tower;

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

// Re-export middleware when tower feature is enabled
#[cfg(feature = "tower")]
pub use middleware::*;

// Re-export stateful module when stateful feature is enabled
#[cfg(feature = "stateful")]
pub use stateful::*;
