//! TLS TCP streaming example with subscription and event streaming.
//!
//! This example demonstrates secure streaming over TLS with:
//! - Encrypted persistent connections
//! - Real-time price updates subscription
//! - System events subscription
//! - Secure event streaming
//!
//! Run this example with:
//! ```bash
//! cargo run --example tls_streaming_example --features tcp-stream-tls,streaming
//! ```
//!
//! Note: You need to generate certificates first:
//! ```bash
//! cd examples/tls_example && ./generate_certs.sh
//! ```

use ash_rpc_core::transport::tcp_tls::{TcpStreamTlsServer, TlsConfig};
use ash_rpc_core::*;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Example: Secure price ticker stream handler
struct SecurePriceTickerHandler;

impl SecurePriceTickerHandler {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl StreamHandler for SecurePriceTickerHandler {
    fn subscription_method(&self) -> &'static str {
        "subscribe_secure_prices"
    }

    async fn subscribe(
        &self,
        params: Option<serde_json::Value>,
        stream_id: StreamId,
    ) -> Result<StreamResponse, Error> {
        let symbol = params
            .as_ref()
            .and_then(|p| p.get("symbol"))
            .and_then(|s| s.as_str())
            .unwrap_or("BTC/USD");

        tracing::info!(
            stream_id = %stream_id,
            symbol = %symbol,
            "secure price ticker subscription created"
        );

        Ok(StreamResponse::success(stream_id, json!(1)))
    }

    async fn unsubscribe(&self, stream_id: &str) -> Result<(), Error> {
        tracing::info!(stream_id = %stream_id, "secure price ticker unsubscribed");
        Ok(())
    }

    async fn start_stream(
        &self,
        stream_id: StreamId,
        params: Option<serde_json::Value>,
        sender: mpsc::UnboundedSender<StreamEvent>,
    ) -> Result<(), Error> {
        let symbol = params
            .as_ref()
            .and_then(|p| p.get("symbol"))
            .and_then(|s| s.as_str())
            .unwrap_or("BTC/USD")
            .to_string();

        // Spawn a task that generates encrypted price updates
        tokio::spawn(async move {
            let mut price = 50000.0;
            let mut sequence = 0u64;

            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;

                // Simulate price changes
                price += (rand::random::<f64>() - 0.5) * 200.0;
                sequence += 1;

                let event_data = json!({
                    "symbol": symbol,
                    "price": format!("{:.2}", price),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "volume_24h": (rand::random::<f64>() * 1000000.0).round(),
                    "market_cap": (price * 19000000.0).round(),
                });

                let event = StreamEvent::new(stream_id.clone(), "secure_price_update", event_data)
                    .with_sequence(sequence);

                if sender.send(event).is_err() {
                    tracing::info!(stream_id = %stream_id, "secure stream closed by client");
                    break;
                }
            }
        });

        Ok(())
    }

    async fn is_active(&self, _stream_id: &str) -> bool {
        true
    }
}

/// Example: Secure market alerts stream handler
struct SecureMarketAlertsHandler;

#[async_trait::async_trait]
impl StreamHandler for SecureMarketAlertsHandler {
    fn subscription_method(&self) -> &'static str {
        "subscribe_market_alerts"
    }

    async fn subscribe(
        &self,
        params: Option<serde_json::Value>,
        stream_id: StreamId,
    ) -> Result<StreamResponse, Error> {
        let threshold = params
            .as_ref()
            .and_then(|p| p.get("threshold"))
            .and_then(|t| t.as_f64())
            .unwrap_or(5.0);

        tracing::info!(
            stream_id = %stream_id,
            threshold = threshold,
            "market alerts subscription created"
        );

        Ok(StreamResponse::success(stream_id, json!(1)))
    }

    async fn unsubscribe(&self, stream_id: &str) -> Result<(), Error> {
        tracing::info!(stream_id = %stream_id, "market alerts unsubscribed");
        Ok(())
    }

    async fn start_stream(
        &self,
        stream_id: StreamId,
        params: Option<serde_json::Value>,
        sender: mpsc::UnboundedSender<StreamEvent>,
    ) -> Result<(), Error> {
        let threshold = params
            .as_ref()
            .and_then(|p| p.get("threshold"))
            .and_then(|t| t.as_f64())
            .unwrap_or(5.0);

        // Spawn a task that generates market alerts
        tokio::spawn(async move {
            let alert_types = vec![
                "PRICE_SURGE",
                "PRICE_DROP",
                "VOLUME_SPIKE",
                "WHALE_MOVEMENT",
            ];
            let mut sequence = 0u64;

            loop {
                tokio::time::sleep(Duration::from_secs(7)).await;

                sequence += 1;
                let alert_type = alert_types[sequence as usize % alert_types.len()];
                let change_pct = (rand::random::<f64>() - 0.5) * 2.0 * threshold;

                let event_data = json!({
                    "type": alert_type,
                    "symbol": "BTC/USD",
                    "change_percent": format!("{:.2}", change_pct),
                    "severity": if change_pct.abs() > threshold { "HIGH" } else { "MEDIUM" },
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "message": format!("{}: {:.2}% change detected", alert_type, change_pct),
                });

                let event = StreamEvent::new(stream_id.clone(), "market_alert", event_data)
                    .with_sequence(sequence);

                if sender.send(event).is_err() {
                    tracing::info!(stream_id = %stream_id, "alert stream closed by client");
                    break;
                }
            }
        });

        Ok(())
    }

    async fn is_active(&self, _stream_id: &str) -> bool {
        true
    }
}

/// Custom message processor with streaming support
struct SecureStreamingProcessor {
    registry: Arc<MethodRegistry>,
    stream_manager: Arc<StreamManager>,
}

impl SecureStreamingProcessor {
    async fn new() -> Self {
        let stream_manager = Arc::new(StreamManager::new());

        // Register stream handlers
        stream_manager
            .register_handler(SecurePriceTickerHandler::new())
            .await;
        stream_manager
            .register_handler(SecureMarketAlertsHandler)
            .await;

        // Create regular RPC methods
        let registry = Arc::new(MethodRegistry::new(register_methods![
            PingMethod,
            ServerInfoMethod,
            ListStreamsMethod,
        ]));

        Self {
            registry,
            stream_manager,
        }
    }
}

#[async_trait::async_trait]
impl MessageProcessor for SecureStreamingProcessor {
    async fn process_message(&self, message: Message) -> Option<Response> {
        match message {
            Message::Request(req) => {
                // Check if this is a streaming request
                if req.method().starts_with("subscribe_") {
                    // Convert to StreamRequest
                    let stream_req = StreamRequest::new(req.method().to_string(), req.id.clone()?);
                    let stream_req = if let Some(params) = req.params {
                        stream_req.with_params(params)
                    } else {
                        stream_req
                    };

                    match self.stream_manager.subscribe(stream_req).await {
                        Ok(stream_resp) => Some(Response::success(
                            json!({
                                "stream_id": stream_resp.stream_id,
                                "status": "subscribed",
                                "encrypted": true
                            }),
                            req.id,
                        )),
                        Err(e) => Some(Response::error(e, req.id)),
                    }
                } else if req.method() == "unsubscribe" {
                    let stream_id = req
                        .params
                        .as_ref()
                        .and_then(|p| p.get("stream_id"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("");

                    match self.stream_manager.unsubscribe(stream_id).await {
                        Ok(_) => Some(Response::success(
                            json!({ "status": "unsubscribed" }),
                            req.id,
                        )),
                        Err(e) => Some(Response::error(e, req.id)),
                    }
                } else if req.method() == "list_active_streams" {
                    let stream_ids = self.stream_manager.active_stream_ids().await;
                    Some(Response::success(
                        json!({
                            "active_streams": stream_ids,
                            "count": stream_ids.len()
                        }),
                        req.id,
                    ))
                } else {
                    // Handle regular RPC methods
                    Some(self.registry.handle_request(req).await)
                }
            }
            Message::Notification(notif) => {
                self.registry.handle_notification(notif).await;
                None
            }
            Message::Response(_) => None,
        }
    }
}

/// Regular RPC method for ping
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

/// Regular RPC method for server info
struct ServerInfoMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for ServerInfoMethod {
    fn method_name(&self) -> &'static str {
        "info"
    }

    async fn call(&self, _params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        rpc_success!(
            json!({
                "server": "ash-rpc secure streaming example",
                "version": "1.0.0",
                "features": ["streaming", "subscriptions", "tls", "encrypted"],
                "security": "TLS 1.3",
            }),
            id
        )
    }
}

/// Method to list active streams
struct ListStreamsMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for ListStreamsMethod {
    fn method_name(&self) -> &'static str {
        "list_active_streams"
    }

    async fn call(&self, _params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        // This is handled by the processor, but we define it here for completeness
        rpc_success!(json!({"streams": []}), id)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    println!("\n=== TLS Encrypted Streaming Server ===\n");

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

    let addr = "127.0.0.1:8443";
    let processor = SecureStreamingProcessor::new().await;
    let stream_manager = Arc::clone(&processor.stream_manager);

    // Load TLS configuration
    println!("🔐 Loading TLS certificates...");
    let tls_config = TlsConfig::from_pem_files(cert_path, key_path)?;
    println!("TLS configuration loaded successfully\n");

    tracing::info!(
        addr = %addr,
        "starting secure TLS streaming server with subscription support"
    );

    // Build and start TLS stream server
    let server = TcpStreamTlsServer::builder(addr)
        .processor(processor)
        .tls_config(tls_config)
        .max_connections(100)
        .build()?;

    // Spawn event broadcaster task
    let stream_manager_clone = Arc::clone(&stream_manager);
    tokio::spawn(async move {
        loop {
            if let Some(event) = stream_manager_clone.next_event().await {
                tracing::debug!(
                    stream_id = %event.stream_id(),
                    method = %event.method,
                    sequence = ?event.sequence(),
                    "broadcasting encrypted event"
                );

                // In a real implementation, you would send this to the specific client
                // For this example, we just log it
                if let Ok(json) = serde_json::to_string_pretty(&event) {
                    tracing::info!("Encrypted Event: {}", json);
                }
            }
        }
    });

    // Spawn a task to periodically report active subscriptions
    let stream_manager_clone2 = Arc::clone(&stream_manager);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(15)).await;

            let count = stream_manager_clone2.active_count().await;
            if count > 0 {
                tracing::info!(active_streams = count, "active encrypted subscriptions");
            }
        }
    });

    println!("TLS Streaming Server Started!");
    println!("Address: {}", addr);
    println!("Encryption: TLS 1.3\n");
    println!("Available methods:");
    println!("  • ping - Test connection");
    println!("  • info - Get server information");
    println!("  • subscribe_secure_prices - Subscribe to encrypted price updates");
    println!(
        "    Example: {{\"jsonrpc\":\"2.0\",\"method\":\"subscribe_secure_prices\",\"params\":{{\"symbol\":\"BTC/USD\"}},\"id\":1}}"
    );
    println!("  • subscribe_market_alerts - Subscribe to encrypted market alerts");
    println!(
        "    Example: {{\"jsonrpc\":\"2.0\",\"method\":\"subscribe_market_alerts\",\"params\":{{\"threshold\":5.0}},\"id\":2}}"
    );
    println!("  • unsubscribe - Unsubscribe from a stream");
    println!(
        "    Example: {{\"jsonrpc\":\"2.0\",\"method\":\"unsubscribe\",\"params\":{{\"stream_id\":\"<stream_id>\"}},\"id\":3}}"
    );
    println!("  • list_active_streams - List all active subscriptions");
    println!("\n⚠️  Note: Use TLS-enabled clients only (e.g., openssl s_client)");
    println!("Connect with: openssl s_client -connect 127.0.0.1:8443");
    println!("=====================================\n");

    // Run the server
    server.run().await?;

    Ok(())
}
