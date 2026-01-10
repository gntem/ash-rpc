use ash_rpc_core::*;
use serde_json::json;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// Initialize tracing subscriber to see structured logs
fn init_tracing() {
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing to see structured audit logs
    init_tracing();

    tracing::info!("starting json-rpc server with audit logging");

    // Create a few test requests with correlation IDs
    let request1 = RequestBuilder::new("ping")
        .id(json!(1))
        .build();
    
    tracing::info!(
        correlation_id = ?request1.correlation_id, 
        method = %request1.method,
        request_id = ?request1.id,
        "created request with auto-generated correlation id"
    );

    // Test with custom correlation ID
    let request2 = RequestBuilder::new("echo")
        .params(json!({"message": "hello world"}))
        .id(json!(2))
        .correlation_id("custom-correlation-id-12345".to_string())
        .build();

    tracing::info!(
        correlation_id = ?request2.correlation_id,
        method = %request2.method,
        request_id = ?request2.id,
        "created request with custom correlation id"
    );

    // Create responses with matching correlation IDs
    let response1 = Response::success(json!("pong"), request1.id.clone());
    tracing::info!(
        correlation_id = ?response1.correlation_id,
        result = ?response1.result,
        response_id = ?response1.id,
        "created response"
    );

    // Demonstrate batch processing
    let batch = vec![
        RequestBuilder::new("ping").id(json!(3)).build(),
        RequestBuilder::new("echo")
            .params(json!({"data": "test"}))
            .id(json!(4))
            .build(),
    ];

    tracing::info!(batch_size = batch.len(), "processing batch request");

    for req in &batch {
        tracing::debug!(
            correlation_id = ?req.correlation_id,
            method = %req.method,
            request_id = ?req.id,
            "batch item"
        );
    }

    tracing::info!("demonstration complete");

    Ok(())
}
