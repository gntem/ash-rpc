# Examples

This directory contains comprehensive examples demonstrating various features of ash-rpc.

## Core Examples

| Example | Description | Run Command |
|---------|-------------|-------------|
| [basic.rs](basic.rs) | Basic method registration with Add and Multiply operations | `cargo run --example basic` |
| [calculator_engine.rs](calculator_engine.rs) | Full-featured calculator with multiple arithmetic operations and typed parameters | `cargo run --example calculator_engine` |
| [tcp_server.rs](tcp_server.rs) | Simple TCP server with inline method registration using closures | `cargo run --example tcp_server --features tcp` |
| [tcp_stream_server.rs](tcp_stream_server.rs) | TCP server with streaming protocol support for persistent connections | `cargo run --example tcp_stream_server --features tcp-stream` |
| [tcp_stream_client.rs](tcp_stream_client.rs) | Client implementation for connecting to TCP streaming servers | `cargo run --example tcp_stream_client --features tcp-stream` |
| [tcp_stateful_server.rs](tcp_stateful_server.rs) | TCP server with shared state across method handlers | `cargo run --example tcp_stateful_server --features tcp-stream,stateful` |
| [dashmap_stateful_server.rs](dashmap_stateful_server.rs) | Stateful server using DashMap for concurrent state management | `cargo run --example dashmap_stateful_server --features tcp-stream,stateful` |

## HTTP/Axum Examples

| Example | Description | Run Command |
|---------|-------------|-------------|
| [axum_server.rs](axum_server.rs) | HTTP JSON-RPC server using Axum web framework | `cargo run --example axum_server --features axum` |
| [axum_simple.rs](axum_simple.rs) | Minimal Axum integration example with basic handlers | `cargo run --example axum_simple --features axum` |
| [axum_stateful_server.rs](axum_stateful_server.rs) | Axum server with application state management | `cargo run --example axum_stateful_server --features axum,stateful` |

## Security Examples

| Example | Description | Run Command |
|---------|-------------|-------------|
| [auth_example.rs](auth_example.rs) | Authentication and authorization with API keys, roles, and IP whitelisting | `cargo run --example auth_example` |
| [auth_with_transport.rs](auth_with_transport.rs) | Integration of authentication with TCP transport layer | `cargo run --example auth_with_transport --features tcp-stream` |
| [security_config_example.rs](security_config_example.rs) | Security configuration with rate limiting, connection limits, and timeouts | `cargo run --example security_config_example --features tcp-stream` |
| [rate_limiting_example.rs](rate_limiting_example.rs) | Request rate limiting and throttling implementation | `cargo run --example rate_limiting_example --features tcp-stream` |
| [error_sanitization_example.rs](error_sanitization_example.rs) | Error message sanitization to prevent sensitive data leakage | `cargo run --example error_sanitization_example` |

## Streaming Examples

| Example | Description | Run Command |
|---------|-------------|-------------|
| [tcp_streaming_example.rs](tcp_streaming_example.rs) | Real-time event streaming over TCP | `cargo run --example tcp_streaming_example --features tcp-stream,streaming` |
| [tcp_streaming_client.rs](tcp_streaming_client.rs) | Client for consuming TCP streaming events | `cargo run --example tcp_streaming_client --features tcp-stream,streaming` |
| [streaming_shutdown_example.rs](streaming_shutdown_example.rs) | Graceful shutdown with active stream cleanup | `cargo run --example streaming_shutdown_example --features tcp-stream,streaming,shutdown` |

## TLS/Encryption Examples

| Example | Description | Run Command |
|---------|-------------|-------------|
| [tls_streaming_example.rs](tls_streaming_example.rs) | TLS-encrypted TCP streaming server | `cargo run --example tls_streaming_example --features tcp-stream-tls,streaming` |
| [tls_streaming_client.rs](tls_streaming_client.rs) | Client for connecting to TLS-encrypted streaming servers | `cargo run --example tls_streaming_client --features tcp-stream-tls,streaming` |
| [tls_tcp_observability_example.rs](tls_tcp_observability_example.rs) | Production-ready TLS TCP server with full observability and separate HTTP metrics endpoint | `cargo run --example tls_tcp_observability_example --features tcp-stream-tls,observability` |

See [tls_example/](tls_example/) for certificate generation and setup instructions.

## Observability Examples

| Example | Description | Run Command |
|---------|-------------|-------------|
| [tracing_example.rs](tracing_example.rs) | Structured logging with tracing integration | `cargo run --example tracing_example --features logging` |
| [healthcheck_example.rs](healthcheck_example.rs) | Health check endpoint implementation | `cargo run --example healthcheck_example` |
| [healthcheck_contrib_example.rs](healthcheck_contrib_example.rs) | Health check using contrib utilities | `cargo run --example healthcheck_contrib_example --features healthcheck` |
| [observable_setup_macro.rs](observable_setup_macro.rs) | Quick observability setup with macro for logging, metrics, and tracing | `cargo run --example observable_setup_macro --features observability` |
| [tls_tcp_observability_example.rs](tls_tcp_observability_example.rs) | Complete production setup: TLS RPC server + separate HTTP metrics/health server | `cargo run --example tls_tcp_observability_example --features tcp-stream-tls,observability` |

See [observability_telemetry/](observability_telemetry/) for complete telemetry stack with Prometheus, Jaeger, and Grafana.

## Advanced Examples

| Example | Description | Run Command |
|---------|-------------|-------------|
| [graceful_shutdown_example.rs](graceful_shutdown_example.rs) | Graceful shutdown with connection draining | `cargo run --example graceful_shutdown_example --features tcp-stream,shutdown` |
| [database_service.rs](database_service.rs) | Database integration patterns with connection pooling | `cargo run --example database_service` |
| [params_validation_demo.rs](params_validation_demo.rs) | Parameter validation and type-safe deserialization | `cargo run --example params_validation_demo` |
| [optional_methods_demo.rs](optional_methods_demo.rs) | Optional method parameters and default values | `cargo run --example optional_methods_demo` |
| [openapi_demo.rs](openapi_demo.rs) | OpenAPI schema generation for JSON-RPC methods | `cargo run --example openapi_demo` |
| [financial_service](financial_service/) | Complete financial data service with authentication, auditing, and database access |  |

## Macro Examples

| Example | Description | Run Command |
|---------|-------------|-------------|
| [macros_demo.rs](macros_demo.rs) | Demonstration of helper macros for method registration | `cargo run --example macros_demo` |
| [new_macros_demo.rs](new_macros_demo.rs) | New macro patterns for cleaner method definitions | `cargo run --example new_macros_demo` |
| [advanced_macros_demo.rs](advanced_macros_demo.rs) | Advanced macro usage for complex method handlers | `cargo run --example advanced_macros_demo` |
| [tcp_server_macro.rs](tcp_server_macro.rs) | TCP server setup using declarative macros | `cargo run --example tcp_server_macro --features tcp` |
| [tcp_stream_server_macro.rs](tcp_stream_server_macro.rs) | TCP streaming server with macro-based configuration | `cargo run --example tcp_stream_server_macro --features tcp-stream` |
| [transport_macros_demo.rs](transport_macros_demo.rs) | Transport layer configuration using macros | `cargo run --example transport_macros_demo --features tcp-stream` |

## Tower Middleware

See [tower_examples/](tower_examples/) for Tower middleware integration examples.

**General pattern:**
```bash
cargo run --example <example_name> --features <required_features>
```
