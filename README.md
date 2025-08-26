# ash-rpc

A comprehensive, modular JSON-RPC 2.0 implementation for Rust with multiple transport layers and extra features.

## Features

- Full implementation with requests, responses, notifications, and batch operations
- TCP, TCP streaming, HTTP via Axum, and Tower middleware
- Fluent API for constructing requests and responses
- Organize and dispatch JSON-RPC methods with automatic routing
- Generate OpenAPI/Swagger specifications from method definitions
- Efficient caching and optimized request handling
- Use only what you need with feature flags
- Support for context-aware method handlers
- CLI tool for generating boilerplate code
- Convenient macros for common response patterns

## Packages

This workspace contains four packages:

- **`ash-rpc-core`** - Core JSON-RPC implementation with transport support
- **`ash-rpc-stateful`** - Stateful JSON-RPC handlers with shared context
- **`ash-rpc-cli`** - Code generation CLI tool
- **`examples`** - Comprehensive examples and demos

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
ash-rpc-core = "0.1.0"
# Optional: for stateful handlers
ash-rpc-stateful = "0.1.0"
```

### Basic Usage

```rust
use ash_rpc_core::*;

fn main() {
    // Create a method registry
    let mut registry = MethodRegistry::new();
    
    // Register a simple method
    registry.register("ping", |_params, id| {
        rpc_success!("pong", id)
    });
    
    // Register a method with parameters
    registry.register("add", |params, id| {
        let nums: Vec<i32> = serde_json::from_value(params.unwrap_or_default())?;
        if nums.len() == 2 {
            rpc_success!(nums[0] + nums[1], id)
        } else {
            rpc_error!(error_codes::INVALID_PARAMS, "Expected 2 numbers", id)
        }
    });
    
    // Handle a request
    let request = Request::new("ping", None, Some(RequestId::Number(1)));
    let response = registry.call("ping", request.params, request.id);
    println!("{}", serde_json::to_string_pretty(&response).unwrap());
}
```

### TCP Server

```rust
use ash_rpc_core::{MethodRegistry, transport::tcp::TcpServerBuilder};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = MethodRegistry::new();
    registry.register("echo", |params, id| {
        rpc_success!(params, id)
    });
    
    let server = TcpServerBuilder::new("127.0.0.1:8080")
        .processor(Arc::new(registry))
        .build()?;
        
    println!("JSON-RPC server listening on 127.0.0.1:8080");
    server.run()?;
    Ok(())
}
```

### HTTP Server with Axum

```rust
use ash_rpc_core::*;
use axum::{http::StatusCode, response::Json, routing::post, Router};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let mut registry = MethodRegistry::new();
    registry.register("greet", |params, id| {
        let name: String = serde_json::from_value(params.unwrap_or_default())?;
        rpc_success!(format!("Hello, {}!", name), id)
    });
    
    let registry = Arc::new(registry);
    
    let app = Router::new()
        .route("/rpc", post({
            let registry = registry.clone();
            move |Json(request): Json<Request>| async move {
                let response = registry.call(&request.method, request.params, request.id);
                (StatusCode::OK, Json(response))
            }
        }));
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("JSON-RPC server listening on http://127.0.0.1:3000/rpc");
    axum::serve(listener, app).await.unwrap();
}
```

### Tower Middleware

```rust
use ash_rpc_core::middleware::JsonRpcLayer;
use tower::{ServiceBuilder, Service};
use axum::{routing::post, Router, Json};

let middleware = ServiceBuilder::new()
    .layer(JsonRpcLayer::new()
        .validate_version(true)
        .require_id(false))
    .service(your_service);
```

### Stateful Handlers

```rust
use ash_rpc_stateful::*;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct AppContext {
    counter: Arc<Mutex<i32>>,
}

impl ServiceContext for AppContext {}

let context = AppContext {
    counter: Arc::new(Mutex::new(0)),
};

let mut registry = StatefulMethodRegistry::new(context);

registry.register_stateful("increment", |ctx, _params, id| {
    let mut counter = ctx.counter.lock().unwrap();
    *counter += 1;
    rpc_success!(*counter, id)
});
```

## Documentation Generation

Generate OpenAPI/Swagger documentation from your JSON-RPC methods:

```rust
use ash_rpc_core::*;

let mut registry = MethodRegistry::new();
registry.register_with_docs(
    "calculate",
    "Performs mathematical calculations",
    Some("Supports basic arithmetic operations"),
    |params, id| {
        // Implementation...
        rpc_success!(42, id)
    }
);

// Generate OpenAPI spec
let docs = registry.render_docs("Calculator API", "1.0.0");
println!("{}", docs);
```

## Examples

The `examples/` directory contains comprehensive examples:

- `basic.rs` - Simple method registration and calling
- `tcp_server.rs` - TCP server implementation
- `axum_server.rs` - HTTP server with Axum
- `tower_http_simple.rs` - Tower middleware with HTTP
- `tower_tcp_simple.rs` - Tower middleware with TCP
- `calculator_engine.rs` - Advanced calculator with macro usage
- `stateful_server.rs` - Stateful context examples
- `docs_demo.rs` - Documentation generation
- `caching_demo.rs` - Performance optimization with caching

## Feature Flags

Configure the library with feature flags:

```toml
[dependencies]
ash-rpc-core = { version = "0.1.0", features = ["tcp", "tower", "docs"] }
```

Available features:

- `tcp` - TCP transport support
- `tcp-stream` - TCP streaming support  
- `tower` - Tower middleware support
- `docs` - Documentation generation
- `macros` - Convenience macros

## CLI Tool

Generate boilerplate code with the CLI tool:

```bash
cargo install ash-rpc-cli

# Generate a new service
ash-rpc-gen --template server --name MyService
```

## Performance

The library includes several performance optimizations:

- **Efficient JSON parsing** with serde
- **Method dispatch caching** for faster lookups
- **Documentation caching** to avoid regeneration
- **Connection pooling** for TCP streams
- **Minimal allocations** in hot paths

## Architecture

```
ash-rpc-core/
├── types.rs          # Core JSON-RPC types
├── builders.rs       # Fluent builders for requests/responses  
├── traits.rs         # Handler and processor traits
├── registry.rs       # Method registration and dispatch
├── transport.rs      # TCP and streaming transports
├── middleware.rs     # Tower middleware integration
├── macros.rs         # Convenience macros
└── utils.rs          # Documentation and utility functions
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass: `cargo test`
5. Submit a pull request

## License

- MIT License ([LICENSE-MIT](LICENSE-MIT))
