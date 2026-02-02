# Tower Middleware Examples

These examples demonstrate how to use ash-rpc with Tower middleware for building composable JSON-RPC services.

## Prerequisites

- `tower` feature enabled in `ash-rpc`

## Examples

### HTTP Simple (`http_simple.rs`)

A Tower-based HTTP JSON-RPC calculator service.

**Run:**

```bash
cargo run --example tower_http_simple --features tower -p ash-rpc
```

### TCP Simple (`tcp_simple.rs`)

A Tower-based TCP streaming JSON-RPC calculator service.

**Run:**

```bash
cargo run --example tower_tcp_simple --features tower -p ash-rpc
```

## Code Structure

```rust
use ash_rpc_core::{Error, Request, Response, error_codes};
use ash_rpc_contrib::JsonRpcLayer;
use tower::{Service, ServiceBuilder};

// Implement Service trait
impl Service<Request> for CalculatorService {
    type Response = Response;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, Error>> + Send>>;
    
    fn call(&mut self, req: Request) -> Self::Future {
        // Handle request
    }
}

// Build service with middleware
let service = ServiceBuilder::new()
    .layer(JsonRpcLayer::new())
    .service(CalculatorService);
```
