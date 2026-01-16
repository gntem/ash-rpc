//! Production-ready example combining TLS TCP server with full observability.
//!
//! This example demonstrates:
//! - Secure TLS-encrypted TCP JSON-RPC server
//! - Full observability: logging, metrics, and distributed tracing
//! - Separate HTTP server exclusively for metrics and health endpoints
//! - No RPC traffic over HTTP - only metrics and health checks
//!
//! Architecture:
//! - Main RPC server: TLS TCP on port 8443
//! - Metrics/Health server: HTTP on port 9090 (Prometheus-compatible)
//!
//! Run this example with:
//! ```bash
//! cargo run --example tls_tcp_observability_example --features tcp-stream-tls,observability
//! ```
//!
//! Generate certificates first:
//! ```bash
//! cd examples/tls_example && ./generate_certs.sh
//! ```
//!
//! Test the server:
//! ```bash
//! # Connect with TLS client
//! openssl s_client -connect 127.0.0.1:8443
//!
//! # Check metrics
//! curl http://localhost:9090/metrics
//!
//! # Check health
//! curl http://localhost:9090/health
//! ```

use ash_rpc_core::transport::tcp_tls::{TcpStreamTlsServer, TlsConfig};
use ash_rpc_core::*;
use ash_rpc_contrib::observability::ObservableProcessor;
use ash_rpc_contrib::observable_setup;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::Arc;

// Define RPC methods
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

struct EchoMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for EchoMethod {
    fn method_name(&self) -> &'static str {
        "echo"
    }

    async fn call(&self, params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        rpc_success!(params.unwrap_or(serde_json::json!(null)), id)
    }
}

struct AddMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for AddMethod {
    fn method_name(&self) -> &'static str {
        "add"
    }

    async fn call(&self, params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        if let Some(params) = params {
            if let Ok(numbers) = serde_json::from_value::<[f64; 2]>(params) {
                let result = numbers[0] + numbers[1];
                rpc_success!(result, id)
            } else {
                rpc_invalid_params!("Expected array of two numbers", id)
            }
        } else {
            rpc_invalid_params!("Missing parameters", id)
        }
    }
}

struct MultiplyMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for MultiplyMethod {
    fn method_name(&self) -> &'static str {
        "multiply"
    }

    async fn call(&self, params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        if let Some(params) = params {
            if let Ok(numbers) = serde_json::from_value::<[f64; 2]>(params) {
                let result = numbers[0] * numbers[1];
                rpc_success!(result, id)
            } else {
                rpc_invalid_params!("Expected array of two numbers", id)
            }
        } else {
            rpc_invalid_params!("Missing parameters", id)
        }
    }
}

struct SlowOperationMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for SlowOperationMethod {
    fn method_name(&self) -> &'static str {
        "slow_operation"
    }

    async fn call(&self, _params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        rpc_success!("completed", id)
    }
}

struct ErrorMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for ErrorMethod {
    fn method_name(&self) -> &'static str {
        "error_test"
    }

    async fn call(&self, _params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        rpc_error!(error_codes::INTERNAL_ERROR, "Simulated error", id)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== TLS TCP Server with Observability ===\n");

    // Check for TLS certificates
    let cert_path = "examples/tls_example/certs/cert.pem";
    let key_path = "examples/tls_example/certs/key.pem";

    if !std::path::Path::new(cert_path).exists() || !std::path::Path::new(key_path).exists() {
        eprintln!(" TLS certificates not found!");
        eprintln!("\nTo generate self-signed certificates for testing, run:");
        eprintln!("  cd examples/tls_example");
        eprintln!("  ./generate_certs.sh\n");
        eprintln!("Then run this example again.\n");
        return Err("Missing TLS certificates".into());
    }

    // Initialize observability stack
    println!("Initializing observability stack...");
    let observability = observable_setup! {
        service_name: "secure-rpc-service",
        metrics_prefix: "secure_rpc",
    };

    let logger = observability.logger();
    let metrics = observability.metrics();

    logger.info("Observability initialized", &[]);
    println!("✓ Observability stack ready\n");

    // Create method registry
    let registry = MethodRegistry::new(register_methods![
        PingMethod,
        EchoMethod,
        AddMethod,
        MultiplyMethod,
        SlowOperationMethod,
        ErrorMethod,
    ]);

    logger.info(
        "Methods registered",
        &[
            ("count", &registry.method_count().to_string().as_str()),
            ("methods", &"ping, echo, add, multiply, slow_operation, error_test"),
        ],
    );

    // Wrap registry with observability
    let observable_processor = ObservableProcessor::builder(Arc::new(registry))
        .with_metrics(Arc::clone(&metrics))
        .with_logger(Arc::clone(&logger))
        .build();

    // Load TLS configuration
    println!("Loading TLS certificates...");
    let tls_config = TlsConfig::from_pem_files(cert_path, key_path)?;
    logger.info("TLS configuration loaded", &[]);
    println!("✓ TLS configuration ready\n");

    // Build TLS TCP server
    let rpc_addr = "127.0.0.1:8443";
    let server = TcpStreamTlsServer::builder(rpc_addr)
        .processor(observable_processor)
        .tls_config(tls_config)
        .max_connections(100)
        .build()?;

    logger.info("TLS TCP server configured", &[("address", &rpc_addr)]);

    // Start HTTP server for metrics and health (separate thread)
    let metrics_addr = "0.0.0.0:9090";
    let metrics_clone = Arc::clone(&metrics);
    let logger_clone = Arc::clone(&logger);

    tokio::spawn(async move {
        let app = Router::new()
            .route("/metrics", get(prometheus_metrics))
            .route("/health", get(health_check))
            .with_state(metrics_clone);

        logger_clone.info(
            "HTTP metrics server starting",
            &[("address", &metrics_addr)],
        );

        let listener = tokio::net::TcpListener::bind(metrics_addr)
            .await
            .expect("Failed to bind metrics server");

        println!("📡 HTTP Metrics Server: http://{}", metrics_addr);
        println!("   • GET /metrics - Prometheus metrics");
        println!("   • GET /health - Health check\n");

        axum::serve(listener, app)
            .await
            .expect("Metrics server failed");
    });

    // Wait for metrics server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Start main TLS RPC server
    println!("TLS JSON-RPC Server: {}", rpc_addr);
    println!("   • Encryption: TLS 1.3");
    println!("   • Max Connections: 100");
    println!("\nAvailable RPC methods:");
    println!("   • ping - Test connection");
    println!("   • echo - Echo parameters back");
    println!("   • add - Add two numbers");
    println!("   • multiply - Multiply two numbers");
    println!("   • slow_operation - Simulated slow operation (500ms)");
    println!("   • error_test - Trigger error for testing");
    println!("\nConnect with TLS client:");
    println!("   openssl s_client -connect {}", rpc_addr);
    println!("\nMetrics available at:");
    println!("   http://localhost:9090/metrics");
    println!("   http://localhost:9090/health");
    println!("\n=====================================\n");

    logger.info("Starting TLS RPC server", &[]);

    // Run the TLS TCP server (blocks)
    server.run().await?;

    Ok(())
}

// Prometheus metrics endpoint handler
async fn prometheus_metrics(
    State(metrics): State<Arc<ash_rpc_contrib::observability::prometheus::PrometheusMetrics>>,
) -> impl IntoResponse {
    match metrics.gather_text() {
        Ok(text) => (StatusCode::OK, text).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to gather metrics: {}", e),
        )
            .into_response(),
    }
}

// Health check endpoint handler
async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "status": "ok",
            "service": "secure-rpc-service",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
}
