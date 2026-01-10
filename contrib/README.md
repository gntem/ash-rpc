# ash-rpc-contrib

Contributed JSON-RPC transport implementations and utilities for ash-rpc.

## Features

This package extends ash-rpc-core with additional transport layers and utilities:

### Transport Implementations

- **HTTP with Axum** - JSON-RPC server integration with the Axum web framework
- **WebSocket** - Full-duplex WebSocket transport with server and client implementations

### Utility Methods

- **Health Check** - Standard health monitoring method for service availability

### Planned Features

- **Caching Middleware** - Response caching for improved performance
- **Rate Limiting** - Request throttling and limiting utilities

## Quick Start

Add the contrib package with desired features:

```sh
cargo add ash-rpc-contrib --features axum,websocket,healthcheck
```

## Feature Flags

Available features:

- `axum` - HTTP transport using Axum web framework
- `websocket` - WebSocket transport for servers and clients
- `healthcheck` - Health check method for service monitoring

## Integration

This package is designed to work seamlessly with ash-rpc-core. Import both packages in your application:

```rust
use ash_rpc_core::*;
use ash_rpc_contrib::*;
```

## License

Licensed under the Apache License, Version 2.0
