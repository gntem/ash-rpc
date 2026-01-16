//! Simple client for testing the TCP streaming server.
//!
//! This client connects to the streaming server and demonstrates:
//! - Subscribing to price updates
//! - Subscribing to system events  
//! - Receiving streamed events
//! - Unsubscribing from streams
//!
//! Run this after starting the tcp_streaming_example server:
//! ```bash
//! cargo run --example tcp_streaming_client --features tcp-stream,streaming
//! ```

use serde_json::json;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== TCP Streaming Client ===\n");
    println!("Connecting to 127.0.0.1:8080...");

    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    println!("Connected!\n");

    // Send ping request
    println!("1. Sending ping request...");
    let ping_request = json!({
        "jsonrpc": "2.0",
        "method": "ping",
        "id": 1
    });
    writer
        .write_all(ping_request.to_string().as_bytes())
        .await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    // Read response
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    println!("   Response: {}\n", response.trim());

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Subscribe to price updates
    println!("2. Subscribing to price updates (BTC/USD)...");
    let subscribe_prices = json!({
        "jsonrpc": "2.0",
        "method": "subscribe_prices",
        "params": {
            "symbol": "BTC/USD"
        },
        "id": 2
    });
    writer
        .write_all(subscribe_prices.to_string().as_bytes())
        .await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    response.clear();
    reader.read_line(&mut response).await?;
    println!("   Response: {}\n", response.trim());

    // Parse stream_id from response
    let price_stream_id: String = serde_json::from_str::<serde_json::Value>(&response)?
        .get("result")
        .and_then(|r| r.get("stream_id"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Subscribe to system events
    println!("3. Subscribing to system events...");
    let subscribe_events = json!({
        "jsonrpc": "2.0",
        "method": "subscribe_events",
        "id": 3
    });
    writer
        .write_all(subscribe_events.to_string().as_bytes())
        .await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    response.clear();
    reader.read_line(&mut response).await?;
    println!("   Response: {}\n", response.trim());

    let events_stream_id: String = serde_json::from_str::<serde_json::Value>(&response)?
        .get("result")
        .and_then(|r| r.get("stream_id"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    println!("4. Listening for streamed events (10 seconds)...\n");
    println!("   Note: In this example, the server logs events but doesn't send them back");
    println!("   In a production implementation, you would receive StreamEvent messages here\n");

    // In a real implementation, the server would push StreamEvent messages
    // For now, we'll just show that we're subscribed
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Unsubscribe from price updates
    if !price_stream_id.is_empty() {
        println!("5. Unsubscribing from price updates...");
        let unsubscribe = json!({
            "jsonrpc": "2.0",
            "method": "unsubscribe",
            "params": {
                "stream_id": price_stream_id
            },
            "id": 4
        });
        writer
            .write_all(unsubscribe.to_string().as_bytes())
            .await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        response.clear();
        reader.read_line(&mut response).await?;
        println!("   Response: {}\n", response.trim());
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Unsubscribe from system events
    if !events_stream_id.is_empty() {
        println!("6. Unsubscribing from system events...");
        let unsubscribe = json!({
            "jsonrpc": "2.0",
            "method": "unsubscribe",
            "params": {
                "stream_id": events_stream_id
            },
            "id": 5
        });
        writer
            .write_all(unsubscribe.to_string().as_bytes())
            .await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        response.clear();
        reader.read_line(&mut response).await?;
        println!("   Response: {}\n", response.trim());
    }

    println!("7. Getting server info...");
    let info_request = json!({
        "jsonrpc": "2.0",
        "method": "info",
        "id": 6
    });
    writer
        .write_all(info_request.to_string().as_bytes())
        .await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    response.clear();
    reader.read_line(&mut response).await?;
    println!("   Response: {}\n", response.trim());

    println!("=== Client Demo Complete ===");
    println!("\nNote: This demonstrates the subscription API.");
    println!("In a full implementation, the server would push StreamEvent");
    println!("messages to clients, which would appear as additional lines");
    println!("in the TCP stream between your requests and responses.");

    Ok(())
}
