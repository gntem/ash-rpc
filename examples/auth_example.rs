//! Example demonstrating authentication/authorization with ash-rpc
//!
//! This example shows how to implement custom auth logic for your RPC service.
//! The library provides only hooks - you implement ALL the auth logic.
//!
//! Run with: cargo run --example auth_example

use ash_rpc_core::*;
use serde_json::json;
use std::collections::HashMap;

// ============================================================================
// Example 1: Simple API Key Authentication
// ============================================================================

/// A simple API key auth policy
/// Expects params to contain: {"api_key": "secret123"}
struct ApiKeyAuth {
    valid_keys: Vec<String>,
}

impl ApiKeyAuth {
    fn new(keys: Vec<String>) -> Self {
        Self { valid_keys: keys }
    }
}

impl auth::AuthPolicy for ApiKeyAuth {
    fn can_access(
        &self,
        method: &str,
        params: Option<&serde_json::Value>,
        _ctx: &auth::ConnectionContext,
    ) -> bool {
        // Allow public methods without auth
        if method == "ping" || method == "health" {
            return true;
        }

        // Check for API key in params
        if let Some(params) = params {
            if let Some(api_key) = params.get("api_key").and_then(|v| v.as_str()) {
                return self.valid_keys.contains(&api_key.to_string());
            }
        }

        false
    }

    fn unauthorized_error(&self, method: &str) -> Response {
        ResponseBuilder::new()
            .error(
                ErrorBuilder::new(
                    -32001,
                    format!(
                        "Unauthorized: valid API key required for method '{}'",
                        method
                    ),
                )
                .build(),
            )
            .build()
    }
}

// ============================================================================
// Example 2: Role-Based Access Control (RBAC)
// ============================================================================

/// User roles for RBAC
#[derive(Debug, Clone, PartialEq)]
enum Role {
    Admin,
    User,
    Guest,
}

/// RBAC policy - user provides role extraction logic
/// Expects params: {"user_token": "...", "method_params": {...}}
struct RbacPolicy {
    method_permissions: HashMap<String, Vec<Role>>,
}

impl RbacPolicy {
    fn new() -> Self {
        let mut perms = HashMap::new();

        // Define which roles can access which methods
        perms.insert("admin.delete_user".to_string(), vec![Role::Admin]);
        perms.insert(
            "user.update_profile".to_string(),
            vec![Role::Admin, Role::User],
        );
        perms.insert(
            "public.get_info".to_string(),
            vec![Role::Admin, Role::User, Role::Guest],
        );

        Self {
            method_permissions: perms,
        }
    }

    // User implements their own token parsing logic
    fn extract_role_from_token(&self, token: &str) -> Option<Role> {
        // In real code, you'd decode JWT, verify signature, extract claims, etc.
        // This is just a demo
        match token {
            "admin_token_123" => Some(Role::Admin),
            "user_token_456" => Some(Role::User),
            "guest_token_789" => Some(Role::Guest),
            _ => None,
        }
    }
}

impl auth::AuthPolicy for RbacPolicy {
    fn can_access(
        &self,
        method: &str,
        params: Option<&serde_json::Value>,
        _ctx: &auth::ConnectionContext,
    ) -> bool {
        // Get required roles for this method
        let required_roles = match self.method_permissions.get(method) {
            Some(roles) => roles,
            None => return false, // Unknown method = deny
        };

        // Extract user's role from params
        if let Some(params) = params {
            if let Some(token) = params.get("user_token").and_then(|v| v.as_str()) {
                if let Some(user_role) = self.extract_role_from_token(token) {
                    return required_roles.contains(&user_role);
                }
            }
        }

        false
    }

    fn unauthorized_error(&self, method: &str) -> Response {
        ResponseBuilder::new()
            .error(
                ErrorBuilder::new(
                    -32002,
                    format!("Forbidden: insufficient permissions for '{}'", method),
                )
                .build(),
            )
            .build()
    }
}

// ============================================================================
// Example 3: IP Whitelist
// ============================================================================

/// IP-based access control
/// In real usage, you'd extract IP from connection metadata
#[allow(dead_code)]
struct IpWhitelist {
    allowed_ips: Vec<String>,
}

#[allow(dead_code)]
impl IpWhitelist {
    fn new(ips: Vec<String>) -> Self {
        Self { allowed_ips: ips }
    }
}

impl auth::AuthPolicy for IpWhitelist {
    fn can_access(
        &self,
        _method: &str,
        _params: Option<&serde_json::Value>,
        ctx: &auth::ConnectionContext,
    ) -> bool {
        // Now we can use the ACTUAL connection context!
        if let Some(addr) = ctx.remote_addr {
            let ip_str = addr.ip().to_string();
            return self
                .allowed_ips
                .iter()
                .any(|allowed| ip_str.starts_with(allowed));
        }
        false
    }
}

// ============================================================================
// Example 4: Rate Limiting
// ============================================================================

use std::collections::HashMap as StdHashMap;
/// Simple rate limiting by counting requests
/// In production, use a proper rate limiting library
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct RateLimiter {
    requests: Arc<Mutex<StdHashMap<String, (u32, Instant)>>>,
    max_requests: u32,
    window: Duration,
}

impl RateLimiter {
    fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            requests: Arc::new(Mutex::new(StdHashMap::new())),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    fn check_rate(&self, user_id: &str) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();

        let (count, last_reset) = requests.entry(user_id.to_string()).or_insert((0, now));

        // Reset if window expired
        if now.duration_since(*last_reset) > self.window {
            *count = 0;
            *last_reset = now;
        }

        // Check limit
        if *count >= self.max_requests {
            return false;
        }

        *count += 1;
        true
    }
}

impl auth::AuthPolicy for RateLimiter {
    fn can_access(
        &self,
        _method: &str,
        params: Option<&serde_json::Value>,
        _ctx: &auth::ConnectionContext,
    ) -> bool {
        // Extract user ID from params
        if let Some(params) = params {
            if let Some(user_id) = params.get("user_id").and_then(|v| v.as_str()) {
                return self.check_rate(user_id);
            }
        }
        false
    }

    fn unauthorized_error(&self, _method: &str) -> Response {
        ResponseBuilder::new()
            .error(ErrorBuilder::new(-32003, "Rate limit exceeded").build())
            .build()
    }
}

// ============================================================================
// Example Methods
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

struct SecretMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for SecretMethod {
    fn method_name(&self) -> &'static str {
        "get_secret"
    }

    async fn call(&self, _params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        rpc_success!(json!({"secret": "The answer is 42"}), id)
    }
}

// ============================================================================
// Demo
// ============================================================================

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Authentication Examples ===\n");

    // -------------------------
    // Example 1: API Key Auth
    // -------------------------
    println!("1. API Key Authentication:");

    let api_auth = ApiKeyAuth::new(vec!["secret123".to_string()]);
    let registry =
        MethodRegistry::new(register_methods![PingMethod, SecretMethod]).with_auth(api_auth);

    // Public method - no auth required
    let response = registry.call("ping", None, Some(json!(1))).await;
    println!("  Ping (no auth): {:?}", response.result);

    // Protected method - valid key
    let response = registry
        .call(
            "get_secret",
            Some(json!({"api_key": "secret123"})),
            Some(json!(2)),
        )
        .await;
    println!("  Secret (valid key): {:?}", response.result);

    // Protected method - invalid key
    let response = registry
        .call(
            "get_secret",
            Some(json!({"api_key": "wrong"})),
            Some(json!(3)),
        )
        .await;
    println!("  Secret (bad key): {:?}\n", response.error);

    // -------------------------
    // Example 2: RBAC
    // -------------------------
    println!("2. Role-Based Access Control:");

    let rbac = RbacPolicy::new();
    let registry = MethodRegistry::empty().with_auth(rbac);

    // Admin can access admin methods
    let can_access = registry
        .call(
            "admin.delete_user",
            Some(json!({"user_token": "admin_token_123"})),
            Some(json!(4)),
        )
        .await;
    println!("  Admin method (admin token): {:?}", can_access.error);

    // User cannot access admin methods
    let can_access = registry
        .call(
            "admin.delete_user",
            Some(json!({"user_token": "user_token_456"})),
            Some(json!(5)),
        )
        .await;
    println!("  Admin method (user token): {:?}\n", can_access.error);

    // -------------------------
    // Example 3: Rate Limiting
    // -------------------------
    println!("3. Rate Limiting (3 requests per 10 seconds):");

    let rate_limiter = RateLimiter::new(3, 10);
    let registry = MethodRegistry::new(register_methods![PingMethod]).with_auth(rate_limiter);

    for i in 1..=5 {
        let response = registry
            .call("ping", Some(json!({"user_id": "user_123"})), Some(json!(i)))
            .await;

        if let Some(err) = response.error {
            println!("  Request {}: RATE LIMITED - {}", i, err.message);
        } else {
            println!("  Request {}: OK", i);
        }
    }
}
