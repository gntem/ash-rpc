use ash_rpc_core::*;
use serde_json::json;
use std::time::Duration;

#[cfg(feature = "tcp")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ash_rpc_core::transport::tcp::{SecurityConfig, TcpServerBuilder};

    // Define a simple ping method
    struct PingMethod;
    
    #[async_trait::async_trait]
    impl JsonRPCMethod for PingMethod {
        fn method_name(&self) -> &'static str { "ping" }
        
        async fn call(
            &self,
            _params: Option<serde_json::Value>,
            id: Option<RequestId>,
        ) -> Response {
            Response::success(json!("pong"), id)
        }
    }

    // Create a simple registry
    let registry = MethodRegistry::new(vec![
        Box::new(PingMethod),
    ]);

    // Configure security settings
    let security_config = SecurityConfig {
        max_connections: 10,              // Allow max 10 concurrent connections
        max_request_size: 1024 * 100,     // 100 KB max request size
        request_timeout: Duration::from_secs(10),  // 10 second request timeout
        idle_timeout: Duration::from_secs(60),     // 60 second idle timeout
    };

    // Create server with security configuration
    let server = TcpServerBuilder::new("127.0.0.1:8080")
        .processor(registry)
        .security_config(security_config)
        .build()?;

    println!("Rate-limited JSON-RPC server listening on 127.0.0.1:8080");
    println!("Security Configuration:");
    println!("  - Max connections: 10");
    println!("  - Max request size: 100 KB");
    println!("  - Request timeout: 10 seconds");
    println!("  - Idle timeout: 60 seconds");
    println!();
    println!("Try connecting with multiple clients to test connection limiting!");
    println!("Try sending large requests to test size limits!");

    server.run()?;
    Ok(())
}

#[cfg(not(feature = "tcp"))]
fn main() {
    eprintln!("This example requires the 'tcp' feature to be enabled.");
    eprintln!("Run with: cargo run --example rate_limiting_example --features tcp");
}
