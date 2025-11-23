/// Example demonstrating a WebSocket JSON-RPC server
///
/// This example shows how to create a WebSocket server that handles
/// JSON-RPC requests over WebSocket connections.
///
/// Run with: cargo run --example websocket_server --features websocket
use ash_rpc_core::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a method registry
    let registry = MethodRegistry::new()
        .register("ping", |_params, id| rpc_success!("pong", id))
        .register("echo", |params, id| {
            if let Some(params) = params {
                rpc_success!(params, id)
            } else {
                rpc_error!(error_codes::INVALID_PARAMS, "Parameters required", id)
            }
        })
        .register("add", |params, id| {
            if let Some(params) = params {
                match serde_json::from_value::<Vec<i32>>(params) {
                    Ok(nums) if nums.len() == 2 => {
                        rpc_success!(nums[0] + nums[1], id)
                    }
                    Ok(_) => {
                        rpc_error!(error_codes::INVALID_PARAMS, "Expected 2 numbers", id)
                    }
                    Err(e) => {
                        rpc_error!(
                            error_codes::INVALID_PARAMS,
                            format!("Invalid parameters: {e}"),
                            id
                        )
                    }
                }
            } else {
                rpc_error!(error_codes::INVALID_PARAMS, "Parameters required", id)
            }
        })
        .register("greet", |params, id| {
            if let Some(params) = params {
                if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
                    rpc_success!(format!("Hello, {name}!"), id)
                } else {
                    rpc_error!(error_codes::INVALID_PARAMS, "Missing 'name' parameter", id)
                }
            } else {
                rpc_error!(error_codes::INVALID_PARAMS, "Parameters required", id)
            }
        })
        .register("get_time", |_params, id| {
            use std::time::{SystemTime, UNIX_EPOCH};
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            rpc_success!(timestamp, id)
        });

    // Create and run WebSocket server
    let server = transport::websocket::WebSocketServer::builder("127.0.0.1:9001")
        .processor(registry)
        .build()?;

    println!("WebSocket JSON-RPC server starting on ws://127.0.0.1:9001");
    println!("Available methods: ping, echo, add, greet, get_time");
    println!("\nExample using websocat:");
    println!(
        "  echo '{{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"id\":1}}' | websocat ws://127.0.0.1:9001"
    );
    println!(
        "  echo '{{\"jsonrpc\":\"2.0\",\"method\":\"add\",\"params\":[5,3],\"id\":2}}' | websocat ws://127.0.0.1:9001"
    );

    server.run().await?;

    Ok(())
}
