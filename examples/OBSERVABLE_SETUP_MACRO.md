# Observability Setup Macro

The `observable_setup!` macro simplifies the initialization of the observability stack (metrics, tracing, and logging) from dozens of lines to just a few.

## Before (Manual Setup)

```rust
use ash_rpc_contrib::logging::{Logger, SlogLoggerImpl};
use ash_rpc_contrib::observability::prometheus::PrometheusMetrics;
use opentelemetry::global;
use opentelemetry_sdk::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
#[cfg(target_os = "linux")]
use prometheus::process_collector::ProcessCollector;
use std::sync::Arc;

// Setup structured logging
let logger: Arc<dyn Logger> = Arc::new(SlogLoggerImpl::new());
logger.info("Starting server", &[]);

// Initialize OpenTelemetry tracer
let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
    .unwrap_or_else(|_| "http://jaeger:4317".to_string());

logger.info("Initializing OpenTelemetry tracer", &[("endpoint", &otlp_endpoint.as_str())]);

let exporter = SpanExporter::builder()
    .with_tonic()
    .with_endpoint(otlp_endpoint)
    .build()
    .expect("Failed to create OTLP exporter");

let tracer_provider = TracerProvider::builder()
    .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
    .with_resource(Resource::new(vec![
        opentelemetry::KeyValue::new("service.name", "ash-rpc-server"),
    ]))
    .build();

global::set_tracer_provider(tracer_provider);
logger.info("OpenTelemetry tracer initialized", &[]);

// Create Prometheus metrics
let metrics = Arc::new(
    PrometheusMetrics::with_prefix("ash_rpc")
        .expect("Failed to create Prometheus metrics"),
);

// Register process metrics (Linux only)
#[cfg(target_os = "linux")]
{
    let process_collector = ProcessCollector::for_self();
    metrics.registry()
        .register(Box::new(process_collector))
        .expect("Failed to register process collector");
    logger.info("Prometheus metrics initialized with process collector", &[]);
}

#[cfg(not(target_os = "linux"))]
logger.info("Prometheus metrics initialized", &[]);
```

**Lines of code: ~50 lines**
**Imports required: 8+**
**Cognitive load: High** - Multiple steps, conditional compilation, error handling

---

## After (With Macro)

```rust
use ash_rpc_contrib::observable_setup;

// 🎯 Everything in one macro call!
let observability = observable_setup! {
    service_name: "ash-rpc-server",
    metrics_prefix: "ash_rpc",
    otlp_endpoint: "http://jaeger:4317",
};

// Extract components
let logger = observability.logger();
let metrics = observability.metrics();

logger.info("Observability stack initialized", &[]);
```

**Lines of code: ~10 lines**
**Imports required: 1**
**Cognitive load: Low** - Single declarative call, all complexity hidden

---

## Usage Variants

### Full Setup (Metrics + Tracing + Logging)
```rust
let observability = observable_setup! {
    service_name: "ash-rpc-server",
    metrics_prefix: "ash_rpc",
    otlp_endpoint: "http://jaeger:4317",
};
```

### Minimal Setup (Just Metrics + Logging)
```rust
let observability = observable_setup! {
    metrics_prefix: "ash_rpc",
};
```

### Without Tracing
```rust
let observability = observable_setup! {
    service_name: "my-service",
    metrics_prefix: "my_app",
};
```

### With Environment Variable
```rust
let observability = observable_setup! {
    service_name: "my-service",
    metrics_prefix: "my_app",
    otlp_endpoint: env!("JAEGER_ENDPOINT"),
};
```

---

## Benefits

1. **Reduced Boilerplate**: 80% reduction in setup code
2. **Single Import**: Only need `observable_setup!` macro
3. **Type Safety**: Compile-time checks on all parameters
4. **Feature-Aware**: Automatically adapts to enabled features
5. **Platform-Aware**: Handles Linux process metrics automatically
6. **Error Handling**: Built-in with clear panic messages
7. **Consistency**: Same setup across all services

---

## Running the Example

```bash
# Build and run
cd contrib
cargo run --example observable_setup_macro --features observability

# Or in the observability_telemetry setup
# Replace the manual setup in src/main.rs lines 20-70 with the macro
```

---

## Migration Guide

To migrate existing code:

1. **Replace imports** - Remove individual component imports, add macro import
2. **Replace setup code** - Replace the entire observability initialization with macro call
3. **Extract components** - Use `.logger()` and `.metrics()` to get components
4. **Test** - Verify metrics, traces, and logs work as before

The macro produces identical runtime behavior to manual setup, just with less code!
