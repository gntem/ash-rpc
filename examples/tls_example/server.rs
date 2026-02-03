use ash_rpc::{
    JsonRPCMethod, MethodRegistry, RequestId, Response,
    transport::tcp_stream_tls::{TcpStreamTlsClient, TcpStreamTlsServer, TlsConfig},
};

// Simple ping method
struct PingMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for PingMethod {
    fn method_name(&self) -> &'static str {
        "ping"
    }

    async fn call(&self, _params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        ash_rpc::rpc_success!("pong", id)
    }
}

// Echo method that returns whatever is sent
struct EchoMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for EchoMethod {
    fn method_name(&self) -> &'static str {
        "echo"
    }

    async fn call(&self, params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        let message = params.unwrap_or_else(|| serde_json::json!(""));
        ash_rpc::rpc_success!(message, id)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== TLS-Enabled JSON-RPC Server Example ===\n");

    // Check if certificates exist
    let cert_path = "examples/tls_example/certs/cert.pem";
    let key_path = "examples/tls_example/certs/key.pem";

    if !std::path::Path::new(cert_path).exists() || !std::path::Path::new(key_path).exists() {
        println!("⚠️  TLS certificates not found!");
        println!("\nTo generate self-signed certificates for testing, run:");
        println!("  cd examples/tls_example");
        println!("  ./generate_certs.sh\n");
        println!("Then run this example again.\n");
        return Err("Missing TLS certificates".into());
    }

    // Create method registry
    let registry = MethodRegistry::new(ash_rpc::register_methods![PingMethod, EchoMethod]);

    println!(
        " Loaded {} methods: {:?}",
        registry.method_count(),
        registry.get_methods()
    );

    // Load TLS configuration
    println!("🔐 Loading TLS certificates...");
    let tls_config = TlsConfig::from_pem_files(cert_path, key_path)?;
    println!(" TLS configuration loaded successfully\n");

    // Start server in background
    let server = TcpStreamTlsServer::builder("127.0.0.1:8443")
        .processor(registry)
        .tls_config(tls_config)
        .build()?;

    println!("Starting TLS server on 127.0.0.1:8443...\n");

    // Run server in background task
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test client connection
    println!("Testing TLS client connection...\n");

    let mut client = TcpStreamTlsClient::connect_insecure("127.0.0.1:8443").await?;
    println!(" TLS handshake successful!\n");

    // Test ping
    println!("Testing 'ping' method:");
    let ping_request = ash_rpc::rpc_request!("ping", 1);
    client.send_request(&ping_request).await?;
    let ping_response = client.recv_response().await?;
    println!("  Request:  {}", serde_json::to_string(&ping_request)?);
    println!(
        "  Response: {}\n",
        serde_json::to_string_pretty(&ping_response)?
    );

    // Test echo
    println!("Testing 'echo' method:");
    let echo_request = ash_rpc::rpc_request!(
        "echo",
        serde_json::json!({"message": "Hello, secure world!", "encrypted": true}),
        2
    );
    client.send_request(&echo_request).await?;
    let echo_response = client.recv_response().await?;
    println!("  Request:  {}", serde_json::to_string(&echo_request)?);
    println!(
        "  Response: {}\n",
        serde_json::to_string_pretty(&echo_response)?
    );

    println!(" All tests passed!");
    println!("\nAll communication was encrypted with TLS!");
    println!("\nServer will continue running. Press Ctrl+C to stop.");

    // Wait for server (or Ctrl+C)
    server_handle.await?;

    Ok(())
}
