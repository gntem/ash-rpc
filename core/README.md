# ash-rpc-core

JSON-RPC 2.0 implementation for Rust.

## Features

- **Full JSON-RPC 2.0 Specification** - Complete support for requests, responses, notifications, and batch operations
- **Multiple Transport Layers** - TCP, TCP streaming, and TLS-encrypted connections
- **Security** - Built-in rate limiting, connection limits, request size controls, and timeout management
- **Structured Audit Logging** - Integrated tracing with correlation IDs for comprehensive audit trails
- **Authentication Hooks** - Flexible authentication/authorization hooks with connection-level context support
- **Error Sanitization** - User-controlled error handling to prevent sensitive data leakage
- **Stateful Handlers** - Context-aware method handlers with shared application state
- **Type-Safe Builders** - Fluent API for constructing requests, responses, and security configurations

## Installation

```bash
# Basic installation
cargo add ash-rpc-core

# With TCP transport
cargo add ash-rpc-core --features tcp

# With TCP streaming
cargo add ash-rpc-core --features tcp-stream

# With TLS support
cargo add ash-rpc-core --features tcp-stream-tls

# With stateful handlers
cargo add ash-rpc-core --features stateful

# With streaming/subscriptions
cargo add ash-rpc-core --features streaming

# With graceful shutdown
cargo add ash-rpc-core --features shutdown

# Multiple features
cargo add ash-rpc-core --features tcp-stream,stateful,streaming,shutdown
```

## Quick Start

### Basic Method Handler

```rust
use ash_rpc_core::*;
use std::pin::Pin;
use std::future::Future;

struct PingMethod;

impl JsonRPCMethod for PingMethod {
    fn method_name(&self) -> &'static str {
        "ping"
    }
    
    fn call<'a>(
        &'a self,
        _params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            rpc_success!("pong", id)
        })
    }
}

#[tokio::main]
async fn main() {
    let registry = MethodRegistry::new(register_methods![PingMethod]);
    
    let response = registry.call("ping", None, Some(serde_json::json!(1))).await;
    println!("{:?}", response);
}
```

### TCP Server with Security

```rust
use ash_rpc_core::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure security settings
    let security_config = SecurityConfigBuilder::new()
        .max_connections(1000)
        .max_request_size(1024 * 1024)  // 1MB
        .request_timeout(std::time::Duration::from_secs(30))
        .build();
    
    let registry = MethodRegistry::new(register_methods![PingMethod]);
    let processor = MessageProcessor::new(registry);
    
    let server = TcpStreamServerBuilder::new("127.0.0.1:8080")
        .processor(processor)
        .security_config(security_config)
        .build()?;
    
    server.run().await?;
    Ok(())
}
```

### Authentication and Authorization

```rust
use ash_rpc_core::*;

struct ApiKeyAuth {
    valid_keys: Vec<String>,
}

impl auth::AuthPolicy for ApiKeyAuth {
    fn can_access(
        &self,
        method: &str,
        params: Option<&serde_json::Value>,
        ctx: &auth::ConnectionContext,
    ) -> bool {
        // Check IP whitelist
        if let Some(addr) = ctx.remote_addr {
            if !self.is_allowed_ip(&addr.ip()) {
                return false;
            }
        }
        
        // Validate API key from params
        params
            .and_then(|p| p.get("api_key"))
            .and_then(|k| k.as_str())
            .map(|k| self.valid_keys.contains(&k.to_string()))
            .unwrap_or(false)
    }
}

let registry = MethodRegistry::new(register_methods![PingMethod])
    .with_auth(ApiKeyAuth { valid_keys: vec!["secret123".to_string()] });
```

### TLS-Encrypted Server

```rust
use ash_rpc_core::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tls_config = TlsConfig::from_pem_files(
        "path/to/cert.pem",
        "path/to/key.pem"
    )?;
    
    let registry = MethodRegistry::new(register_methods![PingMethod]);
    let processor = MessageProcessor::new(registry);
    
    let server = TcpStreamTlsServerBuilder::new("127.0.0.1:8443")
        .processor(processor)
        .tls_config(tls_config)
        .max_connections(500)
        .build()?;
    
    server.run().await?;
    Ok(())
}
```

### Streaming and Subscriptions

Enable real-time event streaming to clients:

```rust
use ash_rpc_core::*;
use tokio::sync::mpsc;

// Implement a stream handler
struct PriceTickerHandler;

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
        Ok(StreamResponse::success(stream_id, serde_json::json!(1)))
    }

    async fn unsubscribe(&self, stream_id: &str) -> Result<(), Error> {
        Ok(())
    }

    async fn start_stream(
        &self,
        stream_id: StreamId,
        params: Option<serde_json::Value>,
        sender: mpsc::UnboundedSender<StreamEvent>,
    ) -> Result<(), Error> {
        // Emit events to the stream
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let event = StreamEvent::new(
                    stream_id.clone(),
                    "price_update",
                    serde_json::json!({"price": 50000.0}),
                );
                if sender.send(event).is_err() {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream_manager = StreamManager::new();
    stream_manager.register_handler(PriceTickerHandler).await;
    
    // Clients can now subscribe with:
    // {"jsonrpc":"2.0","method":"subscribe_prices","id":1}
    
    Ok(())
}
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE-APACHE) for details.
