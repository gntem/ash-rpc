use ash_rpc_contrib::observable_setup;
use ash_rpc_contrib::observability::ObservableProcessor;
use ash_rpc_contrib::observability::prometheus::get_metrics_method;
use ash_rpc_core::*;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // 🎯 SIMPLIFIED SETUP - Everything in one macro call!
    let observability = observable_setup! {
        service_name: "ash-rpc-server",
        metrics_prefix: "ash_rpc",
        otlp_endpoint: "http://jaeger:4317",
    };

    // Extract components
    let logger = observability.logger();
    let metrics = observability.metrics();

    logger.info("Observability stack initialized", &[]);

    // Create method registry
    let mut registry = MethodRegistry::new();

    registry = registry.register("ping", |_params, id| {
        rpc_success!("pong", id)
    });

    registry = registry.register("echo", |params, id| {
        rpc_success!(params.unwrap_or(serde_json::json!(null)), id)
    });

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

    // Add metrics endpoint
    let metrics_clone = Arc::clone(&metrics);
    registry = registry.register("get_metrics", get_metrics_method(metrics_clone));

    // Wrap registry in observable processor
    let observable_processor = ObservableProcessor::builder(Arc::new(registry))
        .with_metrics(Arc::clone(&metrics))
        .with_logger(Arc::clone(&logger))
        .build();

    let processor = Arc::new(observable_processor);

    logger.info("Method registry configured", &[
        ("methods", &"ping, echo, add, multiply, get_metrics"),
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

    axum::serve(listener, app).await.unwrap();
}

// RPC handler
async fn handle_rpc(
    State((processor, metrics)): State<(
        Arc<ObservableProcessor>,
        Arc<ash_rpc_contrib::observability::prometheus::PrometheusMetrics>,
    )>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    metrics.connection_opened();

    let response = if request.is_array() {
        let messages: Vec<Message> = serde_json::from_value(request).unwrap_or_default();
        let mut responses = Vec::new();

        for message in messages {
            if let Some(response) = processor.process_message(message) {
                responses.push(response);
            }
        }

        serde_json::to_value(responses).unwrap()
    } else {
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
        Arc<ash_rpc_contrib::observability::prometheus::PrometheusMetrics>,
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
