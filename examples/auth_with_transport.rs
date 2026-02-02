//! Example demonstrating connection-level authentication with transports
//!
//! This shows how to extract authentication data from the transport layer
//! (TLS certificates, IP addresses, connection metadata) and use it for auth.
//!
//! This example demonstrates the pattern without requiring actual TLS setup.

use ash_rpc::*;
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

// ============================================================================
// Example: IP-Based Authentication Using ConnectionContext
// ============================================================================

/// IP whitelist policy that uses REAL connection context
struct IpWhitelistPolicy {
    allowed_ips: Vec<IpAddr>,
}

impl IpWhitelistPolicy {
    fn new(ips: Vec<IpAddr>) -> Self {
        Self { allowed_ips: ips }
    }
}

impl auth::AuthPolicy for IpWhitelistPolicy {
    fn can_access(
        &self,
        _method: &str,
        _params: Option<&serde_json::Value>,
        ctx: &auth::ConnectionContext,
    ) -> bool {
        // This is the KEY difference - we use REAL connection data
        if let Some(addr) = ctx.remote_addr {
            tracing::info!(
                client_ip = %addr.ip(),
                allowed = ?self.allowed_ips,
                "checking IP whitelist"
            );
            return self.allowed_ips.contains(&addr.ip());
        }

        tracing::warn!("no remote address in context, denying access");
        false
    }

    fn unauthorized_error(&self, _method: &str) -> Response {
        ResponseBuilder::new()
            .error(
                ErrorBuilder::new(
                    ash_rpc::error_codes::INTERNAL_ERROR,
                    "Unauthorized: Your IP is not whitelisted",
                )
                .build(),
            )
            .build()
    }
}

// ============================================================================
// Example: Custom Context Extractor for TLS Certificates
// ============================================================================

/// Example context extractor that could extract TLS peer certificates
///
/// In real usage with TLS, you'd implement this to extract:
/// - Client certificates from TLS handshake
/// - Certificate subject/issuer info
/// - Custom extensions from certificates
/// - Any connection-level authentication data
#[allow(dead_code)]
struct TlsContextExtractor {
    // In real code: certificate validation config, trusted CAs, etc.
}

#[allow(dead_code)]
impl TlsContextExtractor {
    fn new() -> Self {
        Self {}
    }
}

// This shows the PATTERN - implement this trait for your transport
#[async_trait::async_trait]
impl auth::ContextExtractor for TlsContextExtractor {
    async fn extract(
        &self,
        remote_addr: Option<SocketAddr>,
        metadata: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
    ) -> auth::ConnectionContext {
        let mut ctx = auth::ConnectionContext::new();
        ctx.remote_addr = remote_addr;

        // In real TLS implementation, you'd extract certificates here:
        // if let Some(tls_metadata) = metadata {
        //     if let Some(peer_certs) = tls_metadata.downcast_ref::<Vec<Certificate>>() {
        //         ctx.insert("peer_certs".to_string(), peer_certs.clone());
        //
        //         // Extract user ID from certificate subject
        //         if let Some(user_id) = extract_user_from_cert(&peer_certs[0]) {
        //             ctx.insert("user_id".to_string(), user_id);
        //         }
        //     }
        // }

        // For demo purposes, we'll just log
        tracing::info!(
            remote_addr = ?remote_addr,
            has_metadata = metadata.is_some(),
            "extracted connection context"
        );

        ctx
    }
}

// ============================================================================
// Example: Authentication Using Custom Metadata
// ============================================================================

/// Policy that uses custom metadata stored in ConnectionContext
struct CertificateAuthPolicy;

impl auth::AuthPolicy for CertificateAuthPolicy {
    fn can_access(
        &self,
        _method: &str,
        _params: Option<&serde_json::Value>,
        ctx: &auth::ConnectionContext,
    ) -> bool {
        // Check if we have a user_id from the TLS certificate
        if let Some(user_id) = ctx.get::<String>("user_id") {
            tracing::info!(user_id = %user_id, "authenticated via TLS certificate");
            return true;
        }

        // Check if we have peer certificates
        if let Some(_certs) = ctx.get::<Vec<String>>("peer_certs") {
            tracing::info!("found peer certificates in context");
            return true;
        }

        tracing::warn!("no valid certificate found in connection context");
        false
    }

    fn unauthorized_error(&self, _method: &str) -> Response {
        ResponseBuilder::new()
            .error(
                ErrorBuilder::new(
                    ash_rpc::error_codes::INTERNAL_ERROR,
                    "Unauthorized: Valid client certificate required",
                )
                .build(),
            )
            .build()
    }
}

// ============================================================================
// Demo Methods
// ============================================================================

struct PingMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for PingMethod {
    fn method_name(&self) -> &'static str {
        "ping"
    }

    async fn call(&self, _params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        rpc_success!("pong", id)
    }
}

// ============================================================================
// Main Demo
// ============================================================================

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Connection-Level Authentication Demo ===\n");

    // -------------------------
    // Example 1: IP Whitelist with Real Connection Context
    // -------------------------
    println!("1. IP Whitelist (using ConnectionContext):");

    let ip_policy = IpWhitelistPolicy::new(vec![
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
    ]);

    let registry = MethodRegistry::new(register_methods![PingMethod]).with_auth(ip_policy);

    // Simulate connection from allowed IP
    let ctx_allowed = auth::ConnectionContext::with_addr(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        12345,
    ));

    let response = registry
        .call_with_context("ping", None, Some(json!(1)), &ctx_allowed)
        .await;
    println!(
        "  Request from 127.0.0.1: {:?}",
        if response.result.is_some() {
            "ALLOWED"
        } else {
            "DENIED"
        }
    );

    // Simulate connection from blocked IP
    let ctx_blocked = auth::ConnectionContext::with_addr(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
        12345,
    ));

    let response = registry
        .call_with_context("ping", None, Some(json!(2)), &ctx_blocked)
        .await;
    println!(
        "  Request from 10.0.0.1: {:?}",
        if response.result.is_some() {
            "ALLOWED"
        } else {
            "DENIED"
        }
    );
    if let Some(err) = response.error {
        println!("    Error: {}", err.message);
    }

    // -------------------------
    // Example 2: Custom Metadata in Context
    // -------------------------
    println!("\n2. Certificate-Based Auth (using custom metadata):");

    let cert_policy = CertificateAuthPolicy;
    let registry = MethodRegistry::new(register_methods![PingMethod]).with_auth(cert_policy);

    // Simulate connection WITH certificate
    let mut ctx_with_cert = auth::ConnectionContext::with_addr(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)),
        12345,
    ));
    ctx_with_cert.insert("user_id".to_string(), "user123".to_string());
    ctx_with_cert.insert("peer_certs".to_string(), vec!["cert1".to_string()]);

    let response = registry
        .call_with_context("ping", None, Some(json!(3)), &ctx_with_cert)
        .await;
    println!(
        "  With certificate: {:?}",
        if response.result.is_some() {
            "ALLOWED"
        } else {
            "DENIED"
        }
    );

    // Simulate connection WITHOUT certificate
    let ctx_no_cert = auth::ConnectionContext::with_addr(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 51)),
        12345,
    ));

    let response = registry
        .call_with_context("ping", None, Some(json!(4)), &ctx_no_cert)
        .await;
    println!(
        "  Without certificate: {:?}",
        if response.result.is_some() {
            "ALLOWED"
        } else {
            "DENIED"
        }
    );
    if let Some(err) = response.error {
        println!("    Error: {}", err.message);
    }
}
