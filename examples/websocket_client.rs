/// Example demonstrating a WebSocket JSON-RPC client
///
/// This example shows how to create a WebSocket client that connects
/// to a WebSocket JSON-RPC server and sends requests.
///
/// Run with: cargo run --example websocket_client --features websocket
use ash_rpc_core::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to WebSocket JSON-RPC server...");

    // Connect to WebSocket server
    let mut client = transport::websocket::WebSocketClientBuilder::new("ws://127.0.0.1:9001")
        .connect()
        .await?;

    println!("Connected! Sending requests...\n");

    // Send ping request
    let ping_request = RequestBuilder::new("ping")
        .id(serde_json::Value::Number(1.into()))
        .build();

    client.send_message(&Message::Request(ping_request)).await?;
    if let Some(response) = client.recv_response().await? {
        println!(
            "Ping response: {}",
            serde_json::to_string_pretty(&response)?
        );
    }

    // Send add request
    let add_request = RequestBuilder::new("add")
        .params(json!([5, 3]))
        .id(serde_json::Value::Number(2.into()))
        .build();

    client.send_message(&Message::Request(add_request)).await?;
    if let Some(response) = client.recv_response().await? {
        println!(
            "\nAdd response: {}",
            serde_json::to_string_pretty(&response)?
        );
    }

    // Send greet request
    let greet_request = RequestBuilder::new("greet")
        .params(json!({"name": "WebSocket User"}))
        .id(serde_json::Value::Number(3.into()))
        .build();

    client
        .send_message(&Message::Request(greet_request))
        .await?;
    if let Some(response) = client.recv_response().await? {
        println!(
            "\nGreet response: {}",
            serde_json::to_string_pretty(&response)?
        );
    }

    // Send echo request
    let echo_request = RequestBuilder::new("echo")
        .params(json!({"message": "Hello via WebSocket!"}))
        .id(serde_json::Value::Number(4.into()))
        .build();

    client.send_message(&Message::Request(echo_request)).await?;
    if let Some(response) = client.recv_response().await? {
        println!(
            "\nEcho response: {}",
            serde_json::to_string_pretty(&response)?
        );
    }

    // Send get_time request
    let time_request = RequestBuilder::new("get_time")
        .id(serde_json::Value::Number(5.into()))
        .build();

    client.send_message(&Message::Request(time_request)).await?;
    if let Some(response) = client.recv_response().await? {
        println!(
            "\nTime response: {}",
            serde_json::to_string_pretty(&response)?
        );
    }

    // Test error handling with invalid method
    let invalid_request = RequestBuilder::new("invalid_method")
        .id(serde_json::Value::Number(6.into()))
        .build();

    client
        .send_message(&Message::Request(invalid_request))
        .await?;
    if let Some(response) = client.recv_response().await? {
        println!(
            "\nInvalid method response: {}",
            serde_json::to_string_pretty(&response)?
        );
    }

    println!("\nAll requests completed successfully!");

    Ok(())
}
