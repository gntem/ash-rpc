//! Graceful shutdown example for streaming server.
//!
//! Demonstrates graceful shutdown with active subscriptions:
//! - Notifying subscribers before shutdown
//! - Closing streams gracefully
//! - Cleanup hooks for streaming resources
//!
//! Run this example with:
//! ```bash
//! cargo run --example streaming_shutdown_example --features tcp-stream,streaming,shutdown
//! ```

use ash_rpc_core::transport::TcpStreamServer;
use ash_rpc_core::*;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Stream handler that respects shutdown
struct TickerStreamHandler {
    name: String,
}

impl TickerStreamHandler {
    fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait::async_trait]
impl StreamHandler for TickerStreamHandler {
    fn subscription_method(&self) -> &'static str {
        "subscribe_ticker"
    }

    async fn subscribe(
        &self,
        _params: Option<serde_json::Value>,
        stream_id: StreamId,
    ) -> Result<StreamResponse, Error> {
        tracing::info!(stream_id = %stream_id, "ticker subscription created");
        Ok(StreamResponse::success(stream_id, json!(1)))
    }

    async fn unsubscribe(&self, stream_id: &str) -> Result<(), Error> {
        tracing::info!(stream_id = %stream_id, "ticker unsubscribed");
        Ok(())
    }

    async fn start_stream(
        &self,
        stream_id: StreamId,
        _params: Option<serde_json::Value>,
        sender: mpsc::UnboundedSender<StreamEvent>,
    ) -> Result<(), Error> {
        let name = self.name.clone();

        tokio::spawn(async move {
            let mut counter = 0u64;

            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                counter += 1;

                let event_data = json!({
                    "ticker": name,
                    "counter": counter,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                let event =
                    StreamEvent::new(stream_id.clone(), "tick", event_data).with_sequence(counter);

                if sender.send(event).is_err() {
                    tracing::info!(stream_id = %stream_id, "stream closed");
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

/// Streaming processor
struct StreamingProcessor {
    registry: Arc<MethodRegistry>,
    stream_manager: Arc<StreamManager>,
}

impl StreamingProcessor {
    async fn new() -> Self {
        let stream_manager = Arc::new(StreamManager::new());
        stream_manager
            .register_handler(TickerStreamHandler::new("main"))
            .await;

        let registry = Arc::new(MethodRegistry::new(register_methods![PingMethod]));

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
                if req.method().starts_with("subscribe_") {
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
                                "status": "subscribed"
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
                } else {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    println!("\n=== Streaming + Graceful Shutdown Example ===\n");

    // Create shutdown manager
    let shutdown_manager = ShutdownManager::new(
        ShutdownConfigBuilder::new()
            .grace_period(Duration::from_secs(10))
            .build(),
    );

    let processor = StreamingProcessor::new().await;
    let stream_manager = Arc::clone(&processor.stream_manager);

    // Register shutdown hooks
    let stream_manager_hook = Arc::clone(&stream_manager);
    shutdown_manager
        .register_hook(move || {
            let sm = Arc::clone(&stream_manager_hook);
            async move {
                let count = sm.active_count().await;
                tracing::info!(active_streams = count, "Closing active streams...");
                sm.close_all().await;
                tracing::info!("✓ All streams closed");
            }
        })
        .await;

    shutdown_manager
        .register_hook(|| async {
            tracing::info!("Cleaning up streaming resources...");
            tokio::time::sleep(Duration::from_millis(500)).await;
            tracing::info!("✓ Streaming cleanup completed");
        })
        .await;

    let addr = "127.0.0.1:8080";
    let server = TcpStreamServer::builder(addr)
        .processor(processor)
        .build()?;

    println!("Streaming server started on {}", addr);
    println!("\nMethods:");
    println!("  • ping");
    println!("  • subscribe_ticker - Start receiving tick events");
    println!("\nPress Ctrl-C to initiate graceful shutdown");
    println!("Watch streams close gracefully!\n");
    println!("=====================================\n");

    let shutdown_signal = shutdown_manager.signal();
    let server_task = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            tracing::error!(error = %e, "server error");
        }
    });

    // Event broadcaster
    let event_signal = shutdown_signal.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                event = stream_manager.next_event() => {
                    if let Some(event) = event {
                        tracing::debug!(
                            stream_id = %event.stream_id(),
                            sequence = ?event.sequence(),
                            "event"
                        );
                    }
                }
                _ = event_signal.recv() => {
                    tracing::info!("📡 Event broadcaster stopping...");
                    break;
                }
            }
        }
    });

    // Wait for shutdown
    shutdown_manager.wait_for_shutdown().await;

    println!("\nShutdown initiated!");
    println!("Closing streams and draining connections...\n");

    tokio::select! {
        _ = tokio::time::sleep(shutdown_manager.grace_period()) => {
            tracing::warn!("grace period expired");
        }
        _ = server_task => {
            tracing::info!("server stopped");
        }
    }

    println!("\nGraceful shutdown completed!\n");
    Ok(())
}
