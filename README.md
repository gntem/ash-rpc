# ash-rpc

A modular, production-ready JSON-RPC 2.0 implementation for Rust with security, observability, and multiple transport layers.

## Overview

ash-rpc provides a complete JSON-RPC 2.0 ecosystem with enterprise-grade features for building distributed systems. The framework is split into two packages that can be used independently or together.

## Architecture

### ash-rpc-core

Core JSON-RPC 2.0 implementation with comprehensive transport and security features.

**Key Features**

- Full JSON-RPC 2.0 specification support (requests, responses, notifications, batch operations)
- Multiple transport layers: TCP, TCP streaming, TLS-encrypted connections
- Built-in security: rate limiting, connection limits, request size controls, timeout management
- Structured audit logging with correlation IDs for distributed tracing
- Authentication and authorization hooks with connection-level context
- Error sanitization to prevent sensitive data leakage
- Stateful handlers with shared application state
- Streaming and subscription support for real-time events
- Graceful shutdown with connection draining
- Type-safe builders for requests, responses, and configurations

**Installation**

```bash
cargo add ash-rpc-core --features tcp,stateful,streaming,shutdown
```

**Available Features**: `tcp`, `tcp-stream`, `tcp-stream-tls`, `stateful`, `streaming`, `shutdown`

### ash-rpc-contrib

Extended transport implementations and observability utilities for production deployments.

**Key Features**

- HTTP transport with Axum web framework integration
- Health check endpoints for service monitoring
- Trait-based structured logging with tracing backend
- Prometheus metrics (request counters, duration histograms, error tracking)
- OpenTelemetry distributed tracing with Jaeger integration
- Unified observability API combining logging, metrics, and tracing
- Tower middleware integration for HTTP services

**Installation**

```bash
cargo add ash-rpc-contrib --features axum,healthcheck,observability
```

**Available Features**: `axum`, `healthcheck`, `tower`, `logging`, `prometheus`, `opentelemetry`, `observability`

## Quick Start

### Basic Method Handler

```rust
use ash_rpc_core::*;

struct CalculatorMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for CalculatorMethod {
    fn method_name(&self) -> &'static str {
        "calculate"
    }
    
    async fn call(&self, params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        let result = params
            .and_then(|p| p.get("expression"))
            .and_then(|e| e.as_str())
            .map(|expr| format!("Result: {}", expr))
            .unwrap_or_else(|| "Invalid expression".to_string());
        
        rpc_success!(result, id)
    }
}
```

### TCP Server with Security

```rust
use ash_rpc_core::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let security_config = SecurityConfigBuilder::new()
        .max_connections(1000)
        .max_request_size(1024 * 1024)
        .request_timeout(std::time::Duration::from_secs(30))
        .build();
    
    let registry = MethodRegistry::new(register_methods![CalculatorMethod]);
    let processor = MessageProcessor::new(registry);
    
    let server = TcpStreamServerBuilder::new("127.0.0.1:8080")
        .processor(processor)
        .security_config(security_config)
        .build()?;
    
    server.run().await?;
    Ok(())
}
```

### HTTP Server with Axum

```rust
use ash_rpc_core::*;
use ash_rpc_contrib::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = MethodRegistry::new(register_methods![
        CalculatorMethod,
        HealthCheckMethod
    ]);
    
    let processor = MessageProcessor::new(registry);
    
    let app = axum::Router::new()
        .route("/rpc", axum::routing::post(rpc_handler))
        .with_state(processor);
    
    axum::Server::bind(&"127.0.0.1:3000".parse()?)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
```

### Observability Integration

```rust
use ash_rpc_contrib::observable_setup;

let observability = observable_setup! {
    service_name: "calculator-service",
    metrics_prefix: "calculator",
    otlp_endpoint: "http://jaeger:4317",
};

let processor = MessageProcessor::new(registry);
let observable_processor = ObservableProcessor::builder(processor)
    .with_logger(observability.logger())
    .with_metrics(observability.metrics())
    .with_tracing(observability.tracer())
    .build();
```

### Authentication and Authorization

```rust
use ash_rpc_core::*;

struct TokenAuth {
    valid_tokens: Vec<String>,
}

impl auth::AuthPolicy for TokenAuth {
    fn can_access(
        &self,
        method: &str,
        params: Option<&serde_json::Value>,
        ctx: &auth::ConnectionContext,
    ) -> bool {
        params
            .and_then(|p| p.get("token"))
            .and_then(|t| t.as_str())
            .map(|t| self.valid_tokens.contains(&t.to_string()))
            .unwrap_or(false)
    }
}

let registry = MethodRegistry::new(register_methods![CalculatorMethod])
    .with_auth(TokenAuth { 
        valid_tokens: vec!["secret_token".to_string()] 
    });
```

### Streaming and Subscriptions

```rust
use ash_rpc_core::*;
use tokio::sync::mpsc;

struct PriceStreamHandler;

#[async_trait::async_trait]
impl StreamHandler for PriceStreamHandler {
    fn subscription_method(&self) -> &'static str {
        "subscribe_prices"
    }

    async fn start_stream(
        &self,
        stream_id: StreamId,
        params: Option<serde_json::Value>,
        sender: mpsc::UnboundedSender<StreamEvent>,
    ) -> Result<(), Error> {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let event = StreamEvent::new(
                    stream_id.clone(),
                    "price_update",
                    serde_json::json!({"symbol": "BTC", "price": 50000.0}),
                );
                if sender.send(event).is_err() {
                    break;
                }
            }
        });
        Ok(())
    }
}
```

## Examples

View the `examples/` directory for full implementations of servers, clients, authentication, TLS, streaming, and observability setups.

## Documentation

- API documentation: `cargo doc --open`
- Core package: [core/README.md](core/README.md)
- Contrib package: [contrib/README.md](contrib/README.md)

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Project Conventions

This project follows [Conventional Commits](https://www.conventionalcommits.org/) for commit messages.
