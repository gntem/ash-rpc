//! Transport layer implementations for JSON-RPC servers.
//!
//! This module provides various transport protocols for JSON-RPC communication:
//! - **TCP**: Simple one-request-per-connection transport
//! - **TCP Stream**: Persistent connections with multiple requests
//! - **TCP TLS**: Encrypted streaming transport with TLS/rustls
//!

pub mod security;

#[cfg(feature = "tcp")]
pub mod tcp;

#[cfg(feature = "tcp-stream")]
pub mod tcp_stream;

#[cfg(feature = "tcp-stream-tls")]
pub mod tcp_tls;

// Re-export security config for all transports
pub use security::SecurityConfig;

// Re-export TCP transport
#[cfg(feature = "tcp")]
pub use tcp::{TcpServer, TcpServerBuilder};

// Re-export TCP stream transport
#[cfg(feature = "tcp-stream")]
pub use tcp_stream::{TcpStreamClient, TcpStreamClientBuilder, TcpStreamServer, TcpStreamServerBuilder};

// Re-export TLS transport
#[cfg(feature = "tcp-stream-tls")]
pub use tcp_tls::{TcpStreamTlsClient, TcpStreamTlsServer, TcpStreamTlsServerBuilder, TlsConfig};
