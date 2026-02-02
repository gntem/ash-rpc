//! Example demonstrating stateful JSON-RPC with DashMap
//!
//! This example shows how to build a concurrent key-value store using
//! DashMap and stateful JSON-RPC handlers. DashMap provides lock-free
//! concurrent access, making it ideal for high-throughput scenarios.
//!
//! ## Features Demonstrated
//!
//! - Concurrent key-value operations with DashMap
//! - Stateful JSON-RPC service context
//! - Lock-free concurrent access patterns
//! - Error handling with custom error types
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example dashmap_stateful_server --features tcp
//! ```
//!
//! ## Testing with curl
//!
//! Set a value:
//! ```bash
//! echo '{"jsonrpc":"2.0","method":"set","params":{"key":"name","value":"Alice"},"id":1}' | nc localhost 8080
//! ```
//!
//! Get a value:
//! ```bash
//! echo '{"jsonrpc":"2.0","method":"get","params":{"key":"name"},"id":2}' | nc localhost 8080
//! ```
//!
//! Delete a value:
//! ```bash
//! echo '{"jsonrpc":"2.0","method":"delete","params":{"key":"name"},"id":3}' | nc localhost 8080
//! ```
//!
//! List all keys:
//! ```bash
//! echo '{"jsonrpc":"2.0","method":"keys","params":{},"id":4}' | nc localhost 8080
//! ```

#[cfg(feature = "tcp")]
mod example {
    use ash_rpc::stateful::{ServiceContext, StatefulMethodRegistry, StatefulProcessor};
    use ash_rpc::transport::tcp::TcpServer;
    use ash_rpc::{ErrorBuilder, ResponseBuilder};
    use dashmap::DashMap;
    use serde_json::Value;
    use std::sync::Arc;

    #[derive(Debug)]
    pub struct ServiceError(String);

    impl std::fmt::Display for ServiceError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for ServiceError {}

    /// Key-value store service using DashMap for concurrent access
    pub struct KeyValueService {
        store: Arc<DashMap<String, Value>>,
        max_keys: usize,
    }

    impl ServiceContext for KeyValueService {
        type Error = ServiceError;
    }

    impl KeyValueService {
        pub fn new(max_keys: usize) -> Self {
            Self {
                store: Arc::new(DashMap::new()),
                max_keys,
            }
        }

        pub fn set(&self, key: String, value: Value) -> Result<(), ServiceError> {
            if self.store.len() >= self.max_keys && !self.store.contains_key(&key) {
                return Err(ServiceError(format!(
                    "Store has reached maximum capacity of {} keys",
                    self.max_keys
                )));
            }
            self.store.insert(key, value);
            Ok(())
        }

        pub fn get(&self, key: &str) -> Result<Option<Value>, ServiceError> {
            Ok(self.store.get(key).map(|v| v.clone()))
        }

        pub fn delete(&self, key: &str) -> Result<Option<Value>, ServiceError> {
            Ok(self.store.remove(key).map(|(_, v)| v))
        }

        pub fn keys(&self) -> Result<Vec<String>, ServiceError> {
            Ok(self.store.iter().map(|entry| entry.key().clone()).collect())
        }

        pub fn contains(&self, key: &str) -> Result<bool, ServiceError> {
            Ok(self.store.contains_key(key))
        }

        pub fn len(&self) -> Result<usize, ServiceError> {
            Ok(self.store.len())
        }

        pub fn clear(&self) -> Result<usize, ServiceError> {
            let count = self.store.len();
            self.store.clear();
            Ok(count)
        }
    }

    pub fn create_kv_registry() -> StatefulMethodRegistry<KeyValueService> {
        StatefulMethodRegistry::new()
            .register_fn("set", |ctx: &KeyValueService, params, id| {
                let params =
                    params.ok_or_else(|| ServiceError("Missing parameters".to_string()))?;

                let key = params
                    .get("key")
                    .and_then(|k| k.as_str())
                    .ok_or_else(|| ServiceError("Missing 'key' parameter".to_string()))?
                    .to_string();

                let value = params
                    .get("value")
                    .ok_or_else(|| ServiceError("Missing 'value' parameter".to_string()))?
                    .clone();

                match ctx.set(key.clone(), value) {
                    Ok(()) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "success": true,
                            "key": key,
                            "message": "Value stored successfully"
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
            .register_fn("get", |ctx: &KeyValueService, params, id| {
                let params =
                    params.ok_or_else(|| ServiceError("Missing parameters".to_string()))?;

                let key = params
                    .get("key")
                    .and_then(|k| k.as_str())
                    .ok_or_else(|| ServiceError("Missing 'key' parameter".to_string()))?;

                match ctx.get(key) {
                    Ok(Some(value)) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "found": true,
                            "key": key,
                            "value": value
                        }))
                        .id(id)
                        .build()),
                    Ok(None) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "found": false,
                            "key": key,
                            "message": "Key not found"
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
            .register_fn("delete", |ctx: &KeyValueService, params, id| {
                let params =
                    params.ok_or_else(|| ServiceError("Missing parameters".to_string()))?;

                let key = params
                    .get("key")
                    .and_then(|k| k.as_str())
                    .ok_or_else(|| ServiceError("Missing 'key' parameter".to_string()))?;

                match ctx.delete(key) {
                    Ok(Some(value)) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "deleted": true,
                            "key": key,
                            "old_value": value
                        }))
                        .id(id)
                        .build()),
                    Ok(None) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "deleted": false,
                            "key": key,
                            "message": "Key not found"
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
            .register_fn("keys", |ctx: &KeyValueService, _params, id| {
                match ctx.keys() {
                    Ok(keys) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "keys": keys,
                            "count": keys.len()
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
            .register_fn("contains", |ctx: &KeyValueService, params, id| {
                let params =
                    params.ok_or_else(|| ServiceError("Missing parameters".to_string()))?;

                let key = params
                    .get("key")
                    .and_then(|k| k.as_str())
                    .ok_or_else(|| ServiceError("Missing 'key' parameter".to_string()))?;

                match ctx.contains(key) {
                    Ok(exists) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "exists": exists,
                            "key": key
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
            .register_fn("len", |ctx: &KeyValueService, _params, id| {
                match ctx.len() {
                    Ok(count) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "count": count
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
            .register_fn("clear", |ctx: &KeyValueService, _params, id| {
                match ctx.clear() {
                    Ok(count) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "cleared": true,
                            "count": count,
                            "message": format!("Cleared {} keys", count)
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

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting DashMap Stateful JSON-RPC Server");
        println!("Listening on 127.0.0.1:8080");
        println!();
        println!("Available methods:");
        println!("  - set(key, value)    : Store a key-value pair");
        println!("  - get(key)           : Retrieve a value by key");
        println!("  - delete(key)        : Remove a key-value pair");
        println!("  - keys()             : List all keys");
        println!("  - contains(key)      : Check if key exists");
        println!("  - len()              : Get number of stored keys");
        println!("  - clear()            : Remove all keys");
        println!();

        // Create service with max 1000 keys
        let service = KeyValueService::new(1000);

        // Create registry with methods
        let registry = create_kv_registry();

        // Create processor
        let processor = StatefulProcessor::new(service, registry);

        // Start TCP server
        let server = TcpServer::builder("127.0.0.1:8080")
            .processor(processor)
            .build()?;

        server.run()?;

        Ok(())
    }
}

#[cfg(not(feature = "tcp"))]
mod example {
    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("This example requires the 'tcp' feature to be enabled.");
        eprintln!("Run with: cargo run --example dashmap_stateful_server --features tcp");
        std::process::exit(1);
    }
}

#[cfg(feature = "tcp")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    example::run()
}

#[cfg(not(feature = "tcp"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    example::run()
}
