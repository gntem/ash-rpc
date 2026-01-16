# ash-rpc

A modular JSON-RPC 2.0 implementation for Rust.

## Packages

- **`ash-rpc-core`** - Core JSON-RPC implementation with TCP transport support
- **`ash-rpc-contrib`** - Additional utilities and middleware

## Quick Start

```bash
cargo add ash-rpc-core --features tcp
```

### Basic TCP Server

```rust
use ash_rpc_core::*;
use std::pin::Pin;
use std::future::Future;

struct EchoMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for EchoMethod {
    fn method_name(&self) -> &'static str {
        "echo"
    }
    
    async fn call(&self, params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        rpc_success!(params.unwrap_or(serde_json::json!(null)), id)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = MethodRegistry::new(register_methods![EchoMethod]);
    
    // TCP server implementation would go here
    // See examples for complete implementation
    
    Ok(())
}
```

### TCP Streaming

```rust
use ash_rpc_core::*;
use std::pin::Pin;
use std::future::Future;

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
    let registry = MethodRegistry::new(register_methods![PingMethod]);
    
    // TCP streaming implementation would go here
    // See examples for complete implementation
    
    Ok(())
}
```

## Examples

Run the provided examples to see TCP and TCP streaming in action:

```bash
# Basic example
cargo run --example basic

# Calculator engine example
cargo run --example calculator_engine

# TCP server examples (see examples/ directory for complete implementations)
cargo run --example tcp_server
cargo run --example tcp_stream_server
```

Available TCP-related features:

- `tcp` - TCP transport support
- `tcp-stream` - TCP streaming support
- `tokio` - Async runtime support

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)

## Other

**Note**: This project follows [Conventional Commits](https://www.conventionalcommits.org/) for commit messages.
