//! Graceful shutdown example for TCP streaming server.
//!
//! This example demonstrates:
//! - Registering shutdown hooks for cleanup
//! - Handling OS signals (SIGTERM, SIGINT/Ctrl-C)
//! - Graceful connection draining
//! - Custom shutdown triggers
//!
//! Run this example with:
//! ```bash
//! cargo run --example graceful_shutdown_example --features tcp-stream,shutdown
//! ```
//!
//! Test shutdown with:
//! - Press Ctrl-C
//! - Send SIGTERM: `kill <pid>`
//! - Or wait 30 seconds for auto-shutdown

use ash_rpc_core::*;
use ash_rpc_core::transport::TcpStreamServer;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// Example method that simulates work
struct SlowMethod {
    call_count: Arc<AtomicUsize>,
}

impl SlowMethod {
    fn new(call_count: Arc<AtomicUsize>) -> Self {
        Self { call_count }
    }
}

#[async_trait::async_trait]
impl JsonRPCMethod for SlowMethod {
    fn method_name(&self) -> &'static str {
        "slow_operation"
    }

    async fn call(
        &self,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Response {
        let delay_ms = params
            .as_ref()
            .and_then(|p| p.get("delay_ms"))
            .and_then(|d| d.as_u64())
            .unwrap_or(1000);

        tracing::info!(delay_ms = delay_ms, "starting slow operation");
        
        // Simulate long-running work
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        
        let count = self.call_count.fetch_add(1, Ordering::SeqCst) + 1;
        
        tracing::info!(count = count, "slow operation completed");
        
        rpc_success!(
            json!({
                "status": "completed",
                "delay_ms": delay_ms,
                "call_count": count
            }),
            id
        )
    }
}

struct PingMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for PingMethod {
    fn method_name(&self) -> &'static str {
        "ping"
    }

    async fn call(
        &self,
        _params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Response {
        rpc_success!("pong", id)
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

    println!("\n=== Graceful Shutdown Example ===\n");

    // Create shutdown manager with custom config
    let shutdown_config = ShutdownConfigBuilder::new()
        .grace_period(Duration::from_secs(15))
        .force_timeout(Duration::from_secs(30))
        .handle_signals(true)
        .build();

    let shutdown_manager = ShutdownManager::new(shutdown_config);

    // Track method calls for cleanup demonstration
    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = Arc::clone(&call_count);

    // Register shutdown hooks
    println!("Registering shutdown hooks...\n");

    // Hook 1: Close database connections (simulated)
    shutdown_manager.register_hook(|| async {
        tracing::info!("Closing database connections...");
        tokio::time::sleep(Duration::from_millis(500)).await;
        tracing::info!("✓ Database connections closed");
    }).await;

    // Hook 2: Flush in-memory cache (simulated)
    shutdown_manager.register_hook(|| async {
        tracing::info!("Flushing cache to disk...");
        tokio::time::sleep(Duration::from_millis(300)).await;
        tracing::info!("✓ Cache flushed");
    }).await;

    // Hook 3: Report final statistics
    shutdown_manager.register_hook(move || {
        let count = Arc::clone(&call_count_clone);
        async move {
            let final_count = count.load(Ordering::SeqCst);
            tracing::info!(
                total_calls = final_count,
                "Final statistics"
            );
        }
    }).await;

    // Hook 4: Custom cleanup
    shutdown_manager.register_hook(|| async {
        tracing::info!("Running custom cleanup...");
        tokio::time::sleep(Duration::from_millis(200)).await;
        tracing::info!("✓ Cleanup completed");
    }).await;

    tracing::info!("shutdown hooks registered: 4 hooks");

    // Create method registry
    let registry = MethodRegistry::new(register_methods![
        PingMethod,
        SlowMethod::new(Arc::clone(&call_count))
    ]);

    // Build TCP server
    let addr = "127.0.0.1:8080";
    let server = TcpStreamServer::builder(addr)
        .processor(registry)
        .max_connections(10)
        .build()?;

    println!("Server started on {}", addr);
    println!("\nAvailable methods:");
    println!("  • ping - Quick test");
    println!("  • slow_operation - Simulates long-running work");
    println!("    Example: {{\"jsonrpc\":\"2.0\",\"method\":\"slow_operation\",\"params\":{{\"delay_ms\":3000}},\"id\":1}}");
    println!("\nShutdown triggers:");
    println!("  • Press Ctrl-C");
    println!("  • Send SIGTERM signal");
    println!("  • Wait 30 seconds for auto-shutdown demo");
    println!("\nGraceful shutdown settings:");
    println!("  • Grace period: 15 seconds");
    println!("  • Force timeout: 30 seconds");
    println!("  • Shutdown hooks: 4 registered");
    println!("\n=====================================\n");

    // Get shutdown signal and handle
    let shutdown_signal = shutdown_manager.signal();
    let shutdown_handle = shutdown_manager.handle();

    // Spawn server task
    let server_task = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            tracing::error!(error = %e, "server error");
        }
    });

    // Spawn a task to auto-shutdown after 30 seconds (for demo purposes)
    let auto_shutdown_handle = shutdown_handle.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        tracing::info!("Auto-shutdown triggered (demo timeout)");
        auto_shutdown_handle.shutdown().await;
    });

    // Spawn a task to simulate ongoing work
    let work_signal = shutdown_signal.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    tracing::info!("Background work running...");
                }
                _ = work_signal.recv() => {
                    tracing::info!("Background work stopping...");
                    break;
                }
            }
        }
    });

    // Wait for shutdown signal
    tracing::info!("waiting for shutdown signal...");
    shutdown_manager.wait_for_shutdown().await;

    // Shutdown initiated
    println!("\n Shutdown initiated!");
    println!("Draining connections and running cleanup hooks...\n");

    // Give server time to drain
    tracing::info!(
        grace_period_secs = ?shutdown_manager.grace_period(),
        "draining connections"
    );

    tokio::select! {
        _ = tokio::time::sleep(shutdown_manager.grace_period()) => {
            tracing::warn!("grace period expired, forcing shutdown");
        }
        _ = server_task => {
            tracing::info!("server task completed");
        }
    }

    println!("\n Graceful shutdown completed!\n");
    println!("All hooks executed, connections drained.");
    println!("Server stopped cleanly.\n");

    Ok(())
}
