# ash-rpc-contrib

Contributed JSON-RPC transport implementations and utilities for ash-rpc.

## Features

This package extends ash-rpc-core with additional transport layers and utilities:

### Transport Implementations

- **HTTP with Axum** - JSON-RPC server integration with the Axum web framework

### Observability & Telemetry

- **Trait-based Logging** - Structured logging with tracing backend
- **Prometheus Metrics** - Request counters, duration histograms, and error tracking
- **OpenTelemetry Tracing** - Distributed tracing with Jaeger integration
- **Unified Observability** - Combine logging, metrics, and tracing with a single API
- **Setup Macro** - Easy observability stack initialization with `observable_setup!`

### Utility Methods

- **Health Check** - Standard health monitoring method for service availability

### Planned Features

- **Caching Middleware** - Response caching for improved performance
- **Rate Limiting** - Request throttling and limiting utilities

## Quick Start

Add the contrib package with desired features:

```sh
# Basic transport
cargo add ash-rpc-contrib --features axum,healthcheck

# With observability
cargo add ash-rpc-contrib --features axum,healthcheck,observability

# Or individual observability features
cargo add ash-rpc-contrib --features axum,logging,prometheus
```

## Feature Flags

Available features:

- `axum` - HTTP transport using Axum web framework
- `healthcheck` - Health check method for service monitoring
- `tower` - Tower middleware integration
- `logging` - Trait-based structured logging with slog
- `prometheus` - Prometheus metrics collection
- `opentelemetry` - OpenTelemetry distributed tracing
- `observability` - All observability features (logging + prometheus + opentelemetry)

## Observability Usage

### Setup with Macro

The easiest way to initialize observability:

```rust
use ash_rpc_contrib::observable_setup;

let observability = observable_setup! {
    service_name: "my-rpc-service",
    metrics_prefix: "my_app",
    otlp_endpoint: "http://jaeger:4317",
};

// Access components
let logger = observability.logger();
let metrics = observability.metrics();
```

### Manual Setup

For more control, build components individually:

```rust
use ash_rpc_contrib::{
    TracingLogger, PrometheusMetrics, TracingProcessor,
    ObservableProcessor
};
use std::sync::Arc;

// Create logger
let logger = Arc::new(TracingLogger::new());

// Create metrics collector
let metrics = Arc::new(
    PrometheusMetrics::with_prefix("my_app")
        .expect("Failed to create metrics")
);

// Create tracing processor
let tracer = Arc::new(TracingProcessor::new("my-service"));

// Wrap your message processor with observability
let processor = Arc::new(your_message_processor);
let observable = ObservableProcessor::builder(processor)
    .with_logger(logger)
    .with_metrics(metrics)
    .with_tracing(tracer)
    .build();
```

### Feature-Gated Usage

Each observability component is independently optional:

```rust
// Logging only
#[cfg(feature = "logging")]
use ash_rpc_contrib::{Logger, TracingLogger};

// Metrics only
#[cfg(feature = "prometheus")]
use ash_rpc_contrib::PrometheusMetrics;

// Tracing only
#[cfg(feature = "opentelemetry")]
use ash_rpc_contrib::TracingProcessor;
```

## Integration

This package is designed to work seamlessly with ash-rpc-core. Import both packages in your application:

```rust
use ash_rpc_core::*;
use ash_rpc_contrib::*;
```

## License

Licensed under the Apache License, Version 2.0
