//! TCP streaming example demonstrating subscription and event streaming.
//!
//! This example shows how to use the streaming feature to push events to clients
//! over a persistent TCP connection.
//!
//! Run this example with:
//! ```bash
//! cargo run --example tcp_streaming_example --features tcp-stream,streaming
//! ```

use ash_rpc_core::transport::TcpStreamServer;
use ash_rpc_core::*;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Example: Price ticker stream handler
struct PriceTickerHandler;

impl PriceTickerHandler {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl StreamHandler for PriceTickerHandler {
    fn subscription_method(&self) -> &'static str {
        "subscribe_prices"
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
            "price ticker subscription created"
        );

        Ok(StreamResponse::success(stream_id, json!(1)))
    }

    async fn unsubscribe(&self, stream_id: &str) -> Result<(), Error> {
        tracing::info!(stream_id = %stream_id, "price ticker unsubscribed");
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

        // Spawn a task that generates price updates
        tokio::spawn(async move {
            let mut price = 50000.0;
            let mut sequence = 0u64;

            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;

                // Simulate price changes
                price += (rand::random::<f64>() - 0.5) * 100.0;
                sequence += 1;

                let event_data = json!({
                    "symbol": symbol,
                    "price": format!("{:.2}", price),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                let event = StreamEvent::new(stream_id.clone(), "price_update", event_data)
                    .with_sequence(sequence);

                if sender.send(event).is_err() {
                    tracing::info!(stream_id = %stream_id, "stream closed by client");
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

/// Example: System events stream handler
struct SystemEventsHandler;

#[async_trait::async_trait]
impl StreamHandler for SystemEventsHandler {
    fn subscription_method(&self) -> &'static str {
        "subscribe_events"
    }

    async fn subscribe(
        &self,
        _params: Option<serde_json::Value>,
        stream_id: StreamId,
    ) -> Result<StreamResponse, Error> {
        tracing::info!(stream_id = %stream_id, "system events subscription created");
        Ok(StreamResponse::success(stream_id, json!(1)))
    }

    async fn unsubscribe(&self, stream_id: &str) -> Result<(), Error> {
        tracing::info!(stream_id = %stream_id, "system events unsubscribed");
        Ok(())
    }

    async fn start_stream(
        &self,
        stream_id: StreamId,
        _params: Option<serde_json::Value>,
        sender: mpsc::UnboundedSender<StreamEvent>,
    ) -> Result<(), Error> {
        // Spawn a task that generates system events
        tokio::spawn(async move {
            let events = vec!["startup", "health_check", "maintenance", "update"];
            let mut sequence = 0u64;

            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;

                sequence += 1;
                let event_type = events[sequence as usize % events.len()];

                let event_data = json!({
                    "type": event_type,
                    "message": format!("System event: {}", event_type),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                let event = StreamEvent::new(stream_id.clone(), "system_event", event_data)
                    .with_sequence(sequence);

                if sender.send(event).is_err() {
                    tracing::info!(stream_id = %stream_id, "stream closed by client");
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
struct StreamingProcessor {
    registry: Arc<MethodRegistry>,
    stream_manager: Arc<StreamManager>,
}

impl StreamingProcessor {
    async fn new() -> Self {
        let stream_manager = Arc::new(StreamManager::new());

        // Register stream handlers
        stream_manager
            .register_handler(PriceTickerHandler::new())
            .await;
        stream_manager.register_handler(SystemEventsHandler).await;

        // Create regular RPC methods
        let registry = Arc::new(MethodRegistry::new(register_methods![
            PingMethod, InfoMethod,
        ]));

        Self {
            registry,
            stream_manager,
        }
    }
}

#[async_trait::async_trait]
impl MessageProcessor for StreamingProcessor {
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
                        Ok(stream_resp) => {
                            // Convert StreamResponse to Response
                            Some(Response::success(
                                json!({
                                    "stream_id": stream_resp.stream_id,
                                    "status": "subscribed"
                                }),
                                req.id,
                            ))
                        }
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
struct InfoMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for InfoMethod {
    fn method_name(&self) -> &'static str {
        "info"
    }

    async fn call(&self, _params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        rpc_success!(
            json!({
                "server": "ash-rpc streaming example",
                "version": "1.0.0",
                "features": ["streaming", "subscriptions", "tcp-stream"],
            }),
            id
        )
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

    let addr = "127.0.0.1:8080";
    let processor = StreamingProcessor::new().await;
    let stream_manager = Arc::clone(&processor.stream_manager);

    tracing::info!(
        addr = %addr,
        "starting TCP streaming server with subscription support"
    );

    // Build and start TCP stream server
    let server = TcpStreamServer::builder(addr)
        .processor(processor)
        .max_connections(100)
        .build()?;

    // Spawn event broadcaster task
    let stream_manager_clone = Arc::clone(&stream_manager);
    tokio::spawn(async move {
        loop {
            if let Some(event) = stream_manager_clone.next_event().await {
                // In a real implementation, you would send this to the specific client
                // For this example, we just log it
                tracing::debug!(
                    stream_id = %event.stream_id(),
                    method = %event.method,
                    sequence = ?event.sequence(),
                    "broadcasting event"
                );

                // Here you would typically:
                // 1. Look up the client connection by stream_id
                // 2. Serialize the event to JSON
                // 3. Send it over the TCP connection
                // For demonstration, we'll show the JSON
                if let Ok(json) = serde_json::to_string_pretty(&event) {
                    tracing::info!("Event: {}", json);
                }
            }
        }
    });

    // Spawn a task to periodically broadcast to all subscribers
    let stream_manager_clone2 = Arc::clone(&stream_manager);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;

            let count = stream_manager_clone2.active_count().await;
            if count > 0 {
                tracing::info!(active_streams = count, "active subscriptions");
            }
        }
    });

    println!("\n=== TCP Streaming Server Started ===");
    println!("Address: {}", addr);
    println!("\nAvailable methods:");
    println!("  • ping - Regular RPC method");
    println!("  • info - Get server information");
    println!("  • subscribe_prices - Subscribe to price updates");
    println!(
        "    Example: {{\"jsonrpc\":\"2.0\",\"method\":\"subscribe_prices\",\"params\":{{\"symbol\":\"BTC/USD\"}},\"id\":1}}"
    );
    println!("  • subscribe_events - Subscribe to system events");
    println!("    Example: {{\"jsonrpc\":\"2.0\",\"method\":\"subscribe_events\",\"id\":2}}");
    println!("  • unsubscribe - Unsubscribe from a stream");
    println!(
        "    Example: {{\"jsonrpc\":\"2.0\",\"method\":\"unsubscribe\",\"params\":{{\"stream_id\":\"<stream_id>\"}},\"id\":3}}"
    );
    println!("\nConnect with: nc 127.0.0.1 8080");
    println!("=====================================\n");

    // Run the server
    server.run().await?;

    Ok(())
}
