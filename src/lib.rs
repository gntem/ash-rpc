//! # ash-rpc
//!
//! A comprehensive JSON-RPC 2.0 implementation with transport support.
//!
//! ## Features
//!
//! - **Complete JSON-RPC 2.0 support** - Request, response, notification, and batch handling
//! - **Multiple transports** - TCP, TCP streaming, HTTP via Axum, and Tower middleware
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
//! // Create a method registry
//! let registry = MethodRegistry::new(register_methods![PingMethod]);
//! ```

// Core module declarations
pub mod auth;
pub mod builders;
pub mod logger;
pub mod macros;
pub mod registry;
pub mod sanitization;

#[cfg(feature = "audit-logging")]
pub mod audit_logging;

#[cfg(feature = "shutdown")]
pub mod shutdown;

#[cfg(feature = "streaming")]
pub mod streaming;

pub mod traits;
pub mod transports;
pub mod types;

#[cfg(feature = "stateful")]
pub mod stateful;

// Contrib modules at top level
#[cfg(feature = "healthcheck")]
pub mod healthcheck;

#[cfg(feature = "tower")]
pub mod middleware;

#[cfg(any(feature = "logging", feature = "prometheus", feature = "opentelemetry"))]
pub mod observability;

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

// Re-export stateful module when stateful feature is enabled
#[cfg(feature = "stateful")]
pub use stateful::*;

// Re-export streaming module when streaming feature is enabled
#[cfg(feature = "streaming")]
pub use streaming::*;

// Re-export shutdown module when shutdown feature is enabled
#[cfg(feature = "shutdown")]
pub use shutdown::*;

// Re-export audit_logging module when audit-logging feature is enabled
#[cfg(feature = "audit-logging")]
pub use audit_logging::*;

// Re-export transports
pub use transports::SecurityConfig;

#[cfg(feature = "tcp")]
pub use transports::{TcpServer, TcpServerBuilder};

#[cfg(feature = "tcp-stream")]
pub use transports::{
    TcpStreamClient, TcpStreamClientBuilder, TcpStreamServer, TcpStreamServerBuilder,
};

#[cfg(feature = "tcp-stream-tls")]
pub use transports::{
    TcpStreamTlsClient, TcpStreamTlsServer, TcpStreamTlsServerBuilder, TlsConfig,
};

#[cfg(feature = "axum")]
pub use transports::axum;

// Re-export healthcheck when feature is enabled
#[cfg(feature = "healthcheck")]
pub use healthcheck::*;

// Re-export middleware when feature is enabled
#[cfg(feature = "tower")]
pub use middleware::*;

// Re-export observability types when feature is enabled
#[cfg(any(feature = "logging", feature = "prometheus", feature = "opentelemetry"))]
pub use observability::{ObservabilityBuilder, ObservableProcessor};

#[cfg(feature = "prometheus")]
pub use observability::prometheus as obs_prometheus;

#[cfg(feature = "opentelemetry")]
pub use observability::tracing as obs_tracing;

// Re-export tower when feature is enabled
#[cfg(feature = "tower")]
pub use tower;

// Re-export prometheus crate when feature is enabled
#[cfg(feature = "prometheus")]
pub use prometheus;

// Re-export OpenTelemetry crates when feature is enabled
#[cfg(feature = "opentelemetry")]
pub use opentelemetry;

#[cfg(feature = "opentelemetry")]
pub use opentelemetry_otlp;

#[cfg(feature = "opentelemetry")]
pub use opentelemetry_sdk;
