//! Example demonstrating WebSocket and Stateful macros
//!
//! This example shows how to use the convenience macros for creating
//! a stateful WebSocket JSON-RPC server with minimal boilerplate.
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example stateful_websocket_macro --features websocket,stateful
//! ```
//!
//! ## Testing with websocat
//!
//! ```bash
//! # Install websocat if needed: cargo install websocat
//!
//! # Connect and send commands
//! websocat ws://127.0.0.1:9001
//!
//! # Then type:
//! {"jsonrpc":"2.0","method":"increment","params":{"counter":"clicks"},"id":1}
//! {"jsonrpc":"2.0","method":"get","params":{"counter":"clicks"},"id":2}
//! {"jsonrpc":"2.0","method":"reset","params":{"counter":"clicks"},"id":3}
//! ```

#[cfg(all(feature = "websocket", feature = "stateful"))]
mod example {
    use ash_rpc_core::stateful::{ServiceContext, StatefulMethodRegistry};
    use ash_rpc_core::{ErrorBuilder, ResponseBuilder};
    use ash_rpc_core::{rpc_stateful_processor, rpc_stateful_registry, rpc_websocket_server};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    pub struct ServiceError(String);

    impl std::fmt::Display for ServiceError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for ServiceError {}

    /// Simple counter service with multiple named counters
    pub struct CounterService {
        counters: Arc<Mutex<HashMap<String, i64>>>,
    }

    impl ServiceContext for CounterService {
        type Error = ServiceError;
    }

    impl CounterService {
        pub fn new() -> Self {
            Self {
                counters: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        pub fn increment(&self, name: &str) -> Result<i64, ServiceError> {
            let mut counters = self
                .counters
                .lock()
                .map_err(|e| ServiceError(format!("Lock error: {e}")))?;

            let current = counters.get(name).unwrap_or(&0);
            let new_value = current + 1;
            counters.insert(name.to_string(), new_value);
            Ok(new_value)
        }

        pub fn get(&self, name: &str) -> Result<i64, ServiceError> {
            let counters = self
                .counters
                .lock()
                .map_err(|e| ServiceError(format!("Lock error: {e}")))?;
            Ok(*counters.get(name).unwrap_or(&0))
        }

        pub fn reset(&self, name: &str) -> Result<i64, ServiceError> {
            let mut counters = self
                .counters
                .lock()
                .map_err(|e| ServiceError(format!("Lock error: {e}")))?;
            let old_value = *counters.get(name).unwrap_or(&0);
            counters.insert(name.to_string(), 0);
            Ok(old_value)
        }
    }

    fn create_registry() -> StatefulMethodRegistry<CounterService> {
        // Using the rpc_stateful_registry! macro
        rpc_stateful_registry!()
            .register_fn("increment", |ctx: &CounterService, params, id| {
                let name = if let Some(ref params) = params {
                    params
                        .get("counter")
                        .and_then(|n| n.as_str())
                        .unwrap_or("default")
                } else {
                    "default"
                };

                match ctx.increment(name) {
                    Ok(value) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "counter": name,
                            "value": value
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
            .register_fn("get", |ctx: &CounterService, params, id| {
                let name = if let Some(ref params) = params {
                    params
                        .get("counter")
                        .and_then(|n| n.as_str())
                        .unwrap_or("default")
                } else {
                    "default"
                };

                match ctx.get(name) {
                    Ok(value) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "counter": name,
                            "value": value
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
            .register_fn("reset", |ctx: &CounterService, params, id| {
                let name = if let Some(ref params) = params {
                    params
                        .get("counter")
                        .and_then(|n| n.as_str())
                        .unwrap_or("default")
                } else {
                    "default"
                };

                match ctx.reset(name) {
                    Ok(old_value) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "counter": name,
                            "old_value": old_value,
                            "new_value": 0
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
    }

    pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting Stateful WebSocket JSON-RPC Server (using macros!)");
        println!("📍 Listening on ws://127.0.0.1:9001");
        println!();
        println!("Available methods:");
        println!("  - increment(counter) : Increment a named counter");
        println!("  - get(counter)       : Get current value of a counter");
        println!("  - reset(counter)     : Reset a counter to 0");
        println!();

        // Create service context
        let service = CounterService::new();

        // Create method registry
        let registry = create_registry();

        // Create stateful processor using macro
        let processor = rpc_stateful_processor!(service, registry);

        // Start WebSocket server using macro
        rpc_websocket_server!("127.0.0.1:9001", processor).await?;

        Ok(())
    }
}

#[cfg(not(all(feature = "websocket", feature = "stateful")))]
mod example {
    pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("This example requires 'websocket' and 'stateful' features.");
        eprintln!(
            "Run with: cargo run --example stateful_websocket_macro --features websocket,stateful"
        );
        std::process::exit(1);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    example::run().await
}
