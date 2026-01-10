//! Contributed JSON-RPC methods and utilities for ash-rpc

pub mod transports;

#[cfg(feature = "healthcheck")]
pub mod healthcheck;

#[cfg(feature = "tower")]
pub mod middleware;

// Re-export transport modules for convenience
#[cfg(feature = "axum")]
pub use transports::axum;

// Re-export healthcheck for convenience
#[cfg(feature = "healthcheck")]
pub use healthcheck::*;

// Re-export tower middleware for convenience
#[cfg(feature = "tower")]
pub use middleware::*;

// Re-export tower when feature is enabled
#[cfg(feature = "tower")]
pub use tower;
