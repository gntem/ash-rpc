//! Example demonstrating security configuration with builders
//!
//! This example shows how to configure security settings using builders:
//! - Set connection limits
//! - Configure request size limits
//! - Set timeouts with validation
//! - Configure processor capabilities
//! - Use secure defaults
//!
//! Run with: cargo run --example security_config_example --features tcp

#[cfg(feature = "tcp")]
use ash_rpc::{
    MethodRegistry, ProcessorCapabilitiesBuilder, SecurityConfigBuilder,
    transport::tcp::TcpServerBuilder,
};

#[cfg(feature = "tcp")]
fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Security Configuration Example ===\n");

    // Example 1: SecurityConfigBuilder with validation
    println!("1. Creating SecurityConfig with custom limits:");
    let security = SecurityConfigBuilder::new()
        .max_connections(500) // Limit to 500 concurrent connections
        .max_request_size(512 * 1024) // 512 KB max request
        .request_timeout(std::time::Duration::from_secs(15)) // 15 second timeout
        .idle_timeout(std::time::Duration::from_secs(120)) // 2 minute idle timeout
        .build();

    println!("   Max connections: {}", security.max_connections);
    println!("   Max request size: {} bytes", security.max_request_size);
    println!("   Request timeout: {:?}", security.request_timeout);
    println!("   Idle timeout: {:?}\n", security.idle_timeout);

    // Example 2: Using secure defaults
    println!("2. Using secure defaults:");
    let default_security = SecurityConfigBuilder::default().build();
    println!(
        "   Max connections: {} (default)",
        default_security.max_connections
    );
    println!(
        "   Max request size: {} bytes (default)",
        default_security.max_request_size
    );
    println!(
        "   Request timeout: {:?} (default)",
        default_security.request_timeout
    );
    println!(
        "   Idle timeout: {:?} (default)\n",
        default_security.idle_timeout
    );

    // Example 3: ProcessorCapabilities with limits
    println!("3. Configuring ProcessorCapabilities:");
    let capabilities = ProcessorCapabilitiesBuilder::new()
        .supports_batch(true)
        .max_batch_size(Some(50)) // Limit batches to 50 requests
        .max_request_size(Some(256 * 1024)) // 256 KB per request
        .request_timeout_secs(Some(20)) // 20 second timeout
        .build();

    println!("   Supports batch: {}", capabilities.supports_batch);
    println!("   Max batch size: {:?}", capabilities.max_batch_size);
    println!(
        "   Max request size: {:?} bytes",
        capabilities.max_request_size
    );
    println!(
        "   Request timeout: {:?} seconds\n",
        capabilities.request_timeout_secs
    );

    // Example 5: Building a complete server with security
    println!("5. Complete server configuration:");
    let registry = MethodRegistry::new(vec![]);

    let server_result = TcpServerBuilder::new("127.0.0.1:0")
        .processor(registry)
        .security_config(
            SecurityConfigBuilder::new()
                .max_connections(100)
                .max_request_size(256 * 1024)
                .request_timeout(std::time::Duration::from_secs(10))
                .idle_timeout(std::time::Duration::from_secs(60))
                .build(),
        )
        .build();

    match server_result {
        Ok(_server) => {
            println!("   Server configured with security limits");
            println!("   Connection limit: 100");
            println!("   Request size limit: 256 KB");
            println!("   Request timeout: 10 seconds");
            println!("   Idle timeout: 60 seconds");
        }
        Err(e) => {
            println!("   Server configuration failed: {}", e);
        }
    }
}

#[cfg(not(feature = "tcp"))]
fn main() {
    println!("This example requires the 'tcp' feature.");
    println!("Run with: cargo run --example security_config_example --features tcp");
}
