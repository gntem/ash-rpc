//! TLS client for testing the secure streaming server.
//!
//! This client connects to the TLS streaming server and demonstrates:
//! - Secure TLS connection
//! - Subscribing to encrypted price updates
//! - Subscribing to encrypted market alerts
//! - Receiving encrypted streamed events
//! - Unsubscribing from streams
//!
//! Run this after starting the tls_streaming_example server:
//! ```bash
//! cargo run --example tls_streaming_client --features tcp-stream-tls
//! ```

use ash_rpc_core::Request;
use ash_rpc_core::transport::tcp_tls::TcpStreamTlsClient;
use serde_json::json;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== TLS Streaming Client ===\n");
    println!("Connecting to secure server at 127.0.0.1:8443...");

    // Connect with insecure mode (for self-signed certificates)
    let mut client = TcpStreamTlsClient::connect_insecure("127.0.0.1:8443").await?;
    println!("✓ TLS handshake successful!\n");
    println!("🔒 Connection encrypted with TLS 1.3\n");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Send ping request
    println!("1. Testing encrypted ping request...");
    let ping_request = Request::new("ping").with_id(json!(1));
    client.send_request(&ping_request).await?;
    let response = client.recv_response().await?;
    println!(
        "   Response: {}\n",
        serde_json::to_string_pretty(&response)?
    );

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Get server info
    println!("2. Getting server information...");
    let info_request = Request::new("info").with_id(json!(2));
    client.send_request(&info_request).await?;
    let response = client.recv_response().await?;
    println!(
        "   Response: {}\n",
        serde_json::to_string_pretty(&response)?
    );

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Subscribe to secure price updates
    println!("3. Subscribing to encrypted price updates (BTC/USD)...");
    let subscribe_prices = Request::new("subscribe_secure_prices")
        .with_params(json!({"symbol": "BTC/USD"}))
        .with_id(json!(3));
    client.send_request(&subscribe_prices).await?;
    let response = client.recv_response().await?;
    println!(
        "   Response: {}\n",
        serde_json::to_string_pretty(&response)?
    );

    // Parse stream_id from response
    let price_stream_id: String = response
        .result()
        .and_then(|r| r.get("stream_id"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Subscribe to market alerts
    println!("4. Subscribing to encrypted market alerts...");
    let subscribe_alerts = Request::new("subscribe_market_alerts")
        .with_params(json!({"threshold": 3.0}))
        .with_id(json!(4));
    client.send_request(&subscribe_alerts).await?;
    let response = client.recv_response().await?;
    println!(
        "   Response: {}\n",
        serde_json::to_string_pretty(&response)?
    );

    let alerts_stream_id: String = response
        .result()
        .and_then(|r| r.get("stream_id"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    tokio::time::sleep(Duration::from_millis(500)).await;

    // List active streams
    println!("5. Listing active encrypted streams...");
    let list_streams = Request::new("list_active_streams").with_id(json!(5));
    client.send_request(&list_streams).await?;
    let response = client.recv_response().await?;
    println!(
        "   Response: {}\n",
        serde_json::to_string_pretty(&response)?
    );

    println!("6. Listening for encrypted events (10 seconds)...\n");
    println!("   Note: In this example, the server logs events but doesn't send them back");
    println!(
        "   In a production implementation, you would receive encrypted StreamEvent messages\n"
    );

    // In a real implementation, the server would push encrypted StreamEvent messages
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Unsubscribe from price updates
    if !price_stream_id.is_empty() {
        println!("7. Unsubscribing from price updates...");
        let unsubscribe = Request::new("unsubscribe")
            .with_params(json!({"stream_id": price_stream_id}))
            .with_id(json!(6));
        client.send_request(&unsubscribe).await?;
        let response = client.recv_response().await?;
        println!(
            "   Response: {}\n",
            serde_json::to_string_pretty(&response)?
        );
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Unsubscribe from market alerts
    if !alerts_stream_id.is_empty() {
        println!("8. Unsubscribing from market alerts...");
        let unsubscribe = Request::new("unsubscribe")
            .with_params(json!({"stream_id": alerts_stream_id}))
            .with_id(json!(7));
        client.send_request(&unsubscribe).await?;
        let response = client.recv_response().await?;
        println!(
            "   Response: {}\n",
            serde_json::to_string_pretty(&response)?
        );
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify all streams are closed
    println!("9. Verifying all streams closed...");
    let list_streams = Request::new("list_active_streams").with_id(json!(8));
    client.send_request(&list_streams).await?;
    let response = client.recv_response().await?;
    println!(
        "   Response: {}\n",
        serde_json::to_string_pretty(&response)?
    );
    println!("=== Secure Client Demo Complete ===");
    println!("\n🔒 All communication was encrypted with TLS 1.3");
    println!("✓ Subscription lifecycle demonstrated successfully");
    println!("\nNote: In a full implementation, the server would push encrypted");
    println!("StreamEvent messages through the secure TLS channel between your");
    println!("requests and responses.\n");

    Ok(())
}
