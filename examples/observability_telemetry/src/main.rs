use ash_rpc_contrib::logging::{Logger, SlogLoggerImpl};
use ash_rpc_contrib::observability::prometheus::{get_metrics_method, PrometheusMetrics};
use ash_rpc_contrib::observability::ObservableProcessor;
use ash_rpc_core::*;
use ::axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use opentelemetry::global;
use opentelemetry_sdk::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
#[cfg(target_os = "linux")]
use prometheus::process_collector::ProcessCollector;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Setup structured logging with slog
    let logger: Arc<dyn Logger> = Arc::new(SlogLoggerImpl::new());
    logger.info("Starting Observability Telemetry Demo Server", &[]);

    // Initialize OpenTelemetry tracer with OTLP exporter to Jaeger
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://jaeger:4317".to_string());
    
    logger.info("Initializing OpenTelemetry tracer", &[("endpoint", &otlp_endpoint.as_str())]);
    
    // Create OTLP exporter
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint)
        .build()
        .expect("Failed to create OTLP exporter");
    
    // Create tracer provider with batch processor
    let tracer_provider = TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", "ash-rpc-server"),
        ]))
        .build();
    
    global::set_tracer_provider(tracer_provider);
    logger.info("OpenTelemetry tracer initialized", &[]);

    // Create Prometheus metrics with process collector
    let metrics = Arc::new(
        PrometheusMetrics::with_prefix("ash_rpc")
            .expect("Failed to create Prometheus metrics"),
    );

    // Register process metrics (CPU, memory, etc.) - Linux only
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

    // Create method registry with various RPC methods
    let mut registry = MethodRegistry::new();

    // Simple ping method
    registry = registry.register("ping", |_params, id| {
        rpc_success!("pong", id)
    });

    // Echo method
    registry = registry.register("echo", |params, id| {
        rpc_success!(params.unwrap_or(serde_json::json!(null)), id)
    });

    // Math operations
    registry = registry.register("add", |params, id| {
        if let Some(params) = params {
            if let Ok(numbers) = serde_json::from_value::<[f64; 2]>(params) {
                let result = numbers[0] + numbers[1];
                rpc_success!(result, id)
            } else {
                rpc_invalid_params!("Expected array of two numbers", id)
            }
        } else {
            rpc_invalid_params!("Missing parameters", id)
        }
    });

    registry = registry.register("multiply", |params, id| {
        if let Some(params) = params {
            if let Ok(numbers) = serde_json::from_value::<[f64; 2]>(params) {
                let result = numbers[0] * numbers[1];
                rpc_success!(result, id)
            } else {
                rpc_invalid_params!("Expected array of two numbers", id)
            }
        } else {
            rpc_invalid_params!("Missing parameters", id)
        }
    });

    // Simulate slow operation
    registry = registry.register("slow_operation", |_params, id| {
        std::thread::sleep(std::time::Duration::from_millis(500));
        rpc_success!("completed", id)
    });

    // Method that always fails (for error metrics)
    registry = registry.register("always_fails", |_params, id| {
        rpc_error!(
            error_codes::INTERNAL_ERROR,
            "This method always fails for demo purposes",
            id
        )
    });

    // Add metrics endpoint as RPC method
    let metrics_clone = Arc::clone(&metrics);
    registry = registry.register("get_metrics", get_metrics_method(metrics_clone));

    // Wrap registry in observable processor
    let observable_processor = ObservableProcessor::builder(Arc::new(registry))
        .with_metrics(Arc::clone(&metrics))
        .with_logger(Arc::clone(&logger))
        .build();

    let processor = Arc::new(observable_processor);

    logger.info("Method registry configured", &[
        ("methods", &"ping, echo, add, multiply, slow_operation, always_fails, get_metrics"),
    ]);

    // Create Axum router
    let app = Router::new()
        .route("/rpc", post(handle_rpc))
        .route("/health", get(health_check))
        .route("/metrics", get(prometheus_metrics))
        .with_state((processor, metrics));

    let addr = "0.0.0.0:3000";
    logger.info("Server starting", &[("address", &addr)]);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    logger.info("Server ready to accept connections", &[]);

    ::axum::serve(listener, app).await.unwrap();
}

// RPC handler
async fn handle_rpc(
    State((processor, metrics)): State<(
        Arc<ObservableProcessor>,
        Arc<PrometheusMetrics>,
    )>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Track connection
    metrics.connection_opened();

    let response = if request.is_array() {
        // Batch request
        let messages: Vec<Message> = serde_json::from_value(request).unwrap_or_default();
        let mut responses = Vec::new();

        for message in messages {
            if let Some(response) = processor.process_message(message) {
                responses.push(response);
            }
        }

        serde_json::to_value(responses).unwrap()
    } else {
        // Single request
        match serde_json::from_value::<Message>(request) {
            Ok(message) => {
                if let Some(response) = processor.process_message(message) {
                    serde_json::to_value(response).unwrap()
                } else {
                    serde_json::json!(null)
                }
            }
            Err(_) => {
                let error = ResponseBuilder::new()
                    .error(
                        ErrorBuilder::new(error_codes::PARSE_ERROR, "Invalid JSON-RPC request")
                            .build(),
                    )
                    .id(None)
                    .build();
                serde_json::to_value(error).unwrap()
            }
        }
    };

    metrics.connection_closed();

    (StatusCode::OK, Json(response))
}

// Health check endpoint
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

// Prometheus metrics endpoint
async fn prometheus_metrics(
    State((_, metrics)): State<(
        Arc<ObservableProcessor>,
        Arc<PrometheusMetrics>,
    )>,
) -> impl IntoResponse {
    match metrics.gather_text() {
        Ok(text) => (StatusCode::OK, text),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to gather metrics: {}", e),
        ),
    }
}
