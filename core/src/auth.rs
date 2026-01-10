//! Authentication and authorization hooks
//!
//! This module provides minimal traits for implementing authentication
//! and authorization. The library makes NO assumptions about:
//! - How you authenticate (JWT, API keys, OAuth, certificates, etc.)
//! - What your user/identity model looks like
//! - How you authorize (RBAC, ABAC, ACL, custom logic, etc.)
//!
//! You implement the trait, we call your `can_access` method.
//!
//! # Example
//! ```
//! use ash_rpc_core::auth::{AuthPolicy, ConnectionContext};
//!
//! struct MyAuth;
//!
//! impl AuthPolicy for MyAuth {
//!     fn can_access(&self, method: &str, params: Option<&serde_json::Value>, _ctx: &ConnectionContext) -> bool {
//!         // Your logic here - check API keys, JWT tokens, whatever you need
//!         let _ = (method, params);
//!         true
//!     }
//! }
//! ```

use crate::Response;
use std::any::Any;
use std::net::SocketAddr;
use std::sync::Arc;

/// Type alias for auth metadata storage
type AuthMetadata = std::collections::HashMap<String, Arc<dyn Any + Send + Sync>>;

/// Connection context for authentication
///
/// This struct holds metadata about a connection that can be used for
/// authentication and authorization decisions. The library makes NO
/// assumptions about what data you need - store anything in `metadata`.
///
/// # Examples
/// - TLS client certificates
/// - IP addresses for whitelisting
/// - Custom connection-level tokens
/// - Session identifiers
#[derive(Default, Clone)]
pub struct ConnectionContext {
    /// Remote address of the connection
    pub remote_addr: Option<SocketAddr>,

    /// User-defined metadata
    ///
    /// Store any auth-related data here:
    /// - TLS peer certificates
    /// - Extracted user IDs
    /// - Session tokens
    /// - Rate limiting state
    /// - Whatever you need for your auth logic
    pub metadata: AuthMetadata,
}

impl ConnectionContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create context with remote address
    pub fn with_addr(remote_addr: SocketAddr) -> Self {
        Self {
            remote_addr: Some(remote_addr),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Insert typed metadata
    pub fn insert<T: Any + Send + Sync>(&mut self, key: String, value: T) {
        self.metadata.insert(key, Arc::new(value));
    }

    /// Get typed metadata
    pub fn get<T: Any + Send + Sync>(&self, key: &str) -> Option<&T> {
        self.metadata.get(key).and_then(|v| v.downcast_ref::<T>())
    }
}

/// Trait for extracting authentication context from connections
///
/// Implement this to extract auth data from your transport layer.
/// The library will call this when a new connection is established.
///
/// # What you can extract:
/// - TLS client certificates for mutual TLS authentication
/// - IP addresses for whitelisting/geoblocking
/// - Custom connection-level authentication tokens
/// - Any connection metadata you need for auth decisions
///
/// # Example: TLS Certificate Extraction
/// ```text
/// use ash_rpc_core::auth::{ContextExtractor, ConnectionContext};
///
/// struct TlsContextExtractor;
///
/// #[async_trait::async_trait]
/// impl ContextExtractor for TlsContextExtractor {
///     async fn extract(&self, stream: &tokio_rustls::server::TlsStream<tokio::net::TcpStream>) -> ConnectionContext {
///         let mut ctx = ConnectionContext::new();
///         
///         // Extract TLS peer certificates
///         if let Some(certs) = stream.get_ref().1.peer_certificates() {
///             ctx.insert("peer_certs".to_string(), certs.clone());
///         }
///         
///         // Extract client IP
///         if let Ok(addr) = stream.get_ref().0.peer_addr() {
///             ctx.remote_addr = Some(addr);
///         }
///         
///         ctx
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait ContextExtractor: Send + Sync {
    /// Extract connection context for authentication
    ///
    /// This is called once when a connection is established.
    /// The returned context is passed to the auth policy for each request.
    ///
    /// # Arguments
    /// * `remote_addr` - Remote socket address of the connection
    /// * `metadata` - Optional transport-specific metadata (e.g., TLS session data)
    ///
    /// # Returns
    /// A `ConnectionContext` with whatever data you need for auth
    async fn extract(
        &self,
        remote_addr: Option<SocketAddr>,
        metadata: Option<Arc<dyn Any + Send + Sync>>,
    ) -> ConnectionContext;
}

/// Default context extractor that only captures the remote address
pub struct DefaultContextExtractor;

#[async_trait::async_trait]
impl ContextExtractor for DefaultContextExtractor {
    async fn extract(
        &self,
        remote_addr: Option<SocketAddr>,
        _metadata: Option<Arc<dyn Any + Send + Sync>>,
    ) -> ConnectionContext {
        ConnectionContext {
            remote_addr,
            metadata: std::collections::HashMap::new(),
        }
    }
}

/// Trait for implementing authentication/authorization checks
///
/// Implement this to control access to your JSON-RPC methods.
/// The library will call `can_access` before executing methods.
///
/// You decide what "access" means - it could be:
/// - Checking an API key in request params
/// - Validating a JWT token
/// - Verifying client certificates (via ConnectionContext)
/// - Role-based checks
/// - Rate limiting
/// - IP whitelisting (via ConnectionContext)
/// - Anything else you need
pub trait AuthPolicy: Send + Sync {
    /// Check if a request should be allowed to proceed
    ///
    /// # Arguments
    /// * `method` - The JSON-RPC method being called
    /// * `params` - Optional parameters from the request
    /// * `ctx` - Connection context (IP, TLS certs, custom metadata)
    ///
    /// # Returns
    /// `true` if the request should proceed, `false` to deny
    ///
    /// # Example: Using Connection Context
    /// ```text
    /// fn can_access(
    ///     &self,
    ///     method: &str,
    ///     params: Option<&serde_json::Value>,
    ///     ctx: &ConnectionContext,
    /// ) -> bool {
    ///     // Check IP whitelist
    ///     if let Some(addr) = ctx.remote_addr {
    ///         if !self.is_ip_allowed(&addr.ip()) {
    ///             return false;
    ///         }
    ///     }
    ///     
    ///     // Check TLS client certificate
    ///     if let Some(certs) = ctx.get::<Vec<Certificate>>("peer_certs") {
    ///         return self.validate_client_cert(certs);
    ///     }
    ///     
    ///     // Check token in params
    ///     let token = params
    ///         .and_then(|p| p.get("auth_token"))
    ///         .and_then(|t| t.as_str());
    ///     
    ///     self.validate_token(token)
    /// }
    /// ```
    fn can_access(
        &self,
        method: &str,
        params: Option<&serde_json::Value>,
        ctx: &ConnectionContext,
    ) -> bool;

    /// Optional: Get the unauthorized error response
    ///
    /// Override this if you want custom error messages for denied requests.
    /// Default returns a generic "Unauthorized" error.
    fn unauthorized_error(&self, method: &str) -> Response {
        let _ = method;
        crate::ResponseBuilder::new()
            .error(crate::Error::new(-32001, "Unauthorized"))
            .id(None)
            .build()
    }
}

/// Helper: Always allow all requests (no authentication)
///
/// Use this as a placeholder or for development/testing.
pub struct AllowAll;

impl AuthPolicy for AllowAll {
    fn can_access(
        &self,
        _method: &str,
        _params: Option<&serde_json::Value>,
        _ctx: &ConnectionContext,
    ) -> bool {
        true
    }
}

/// Helper: Deny all requests
///
/// Useful for maintenance mode or testing denial paths.
pub struct DenyAll;

impl AuthPolicy for DenyAll {
    fn can_access(
        &self,
        _method: &str,
        _params: Option<&serde_json::Value>,
        _ctx: &ConnectionContext,
    ) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_all() {
        let policy = AllowAll;
        let ctx = ConnectionContext::new();
        assert!(policy.can_access("any_method", None, &ctx));
        assert!(policy.can_access(
            "another_method",
            Some(&serde_json::json!({"key": "value"})),
            &ctx
        ));
    }

    #[test]
    fn test_deny_all() {
        let policy = DenyAll;
        let ctx = ConnectionContext::new();
        assert!(!policy.can_access("any_method", None, &ctx));
        assert!(!policy.can_access(
            "another_method",
            Some(&serde_json::json!({"key": "value"})),
            &ctx
        ));
    }

    #[test]
    fn test_connection_context() {
        let mut ctx = ConnectionContext::new();

        // Insert and retrieve typed metadata
        ctx.insert("user_id".to_string(), 42u64);
        assert_eq!(ctx.get::<u64>("user_id"), Some(&42));

        // Wrong type returns None
        assert_eq!(ctx.get::<String>("user_id"), None);

        // Non-existent key returns None
        assert_eq!(ctx.get::<u64>("other"), None);
    }

    #[test]
    fn test_connection_context_with_addr() {
        use std::net::{IpAddr, Ipv4Addr};
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let ctx = ConnectionContext::with_addr(addr);

        assert_eq!(ctx.remote_addr, Some(addr));
        assert_eq!(ctx.metadata.len(), 0);
    }

    #[test]
    fn test_connection_context_default() {
        let ctx = ConnectionContext::default();
        assert!(ctx.remote_addr.is_none());
        assert_eq!(ctx.metadata.len(), 0);
    }

    #[test]
    fn test_connection_context_multiple_metadata() {
        let mut ctx = ConnectionContext::new();

        ctx.insert("user_id".to_string(), 123u64);
        ctx.insert("username".to_string(), String::from("alice"));
        ctx.insert("is_admin".to_string(), true);

        assert_eq!(ctx.get::<u64>("user_id"), Some(&123));
        assert_eq!(ctx.get::<String>("username"), Some(&String::from("alice")));
        assert_eq!(ctx.get::<bool>("is_admin"), Some(&true));
    }

    #[test]
    fn test_allow_all_unauthorized_error() {
        let policy = AllowAll;
        let response = policy.unauthorized_error("test_method");
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32001);
        assert_eq!(error.message, "Unauthorized");
    }

    #[test]
    fn test_deny_all_unauthorized_error() {
        let policy = DenyAll;
        let response = policy.unauthorized_error("blocked_method");
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_default_context_extractor() {
        use std::net::{IpAddr, Ipv4Addr};
        let extractor = DefaultContextExtractor;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 9000);

        let ctx = extractor.extract(Some(addr), None).await;
        assert_eq!(ctx.remote_addr, Some(addr));
        assert_eq!(ctx.metadata.len(), 0);
    }

    #[tokio::test]
    async fn test_default_context_extractor_no_addr() {
        let extractor = DefaultContextExtractor;
        let ctx = extractor.extract(None, None).await;
        assert!(ctx.remote_addr.is_none());
    }

    #[tokio::test]
    async fn test_default_context_extractor_with_metadata() {
        use std::net::{IpAddr, Ipv4Addr};
        let extractor = DefaultContextExtractor;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 3000);
        let metadata: Arc<dyn Any + Send + Sync> = Arc::new(String::from("test"));

        let ctx = extractor.extract(Some(addr), Some(metadata)).await;
        assert_eq!(ctx.remote_addr, Some(addr));
    }

    #[test]
    fn test_connection_context_clone() {
        let mut ctx1 = ConnectionContext::new();
        ctx1.insert("key".to_string(), 100u32);

        let ctx2 = ctx1.clone();
        assert_eq!(ctx2.get::<u32>("key"), Some(&100));
    }
}
