use ::axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
};
use ash_rpc_contrib::observability::ObservableProcessor;
use ash_rpc_contrib::observable_setup;
use ash_rpc_core::*;
use std::sync::Arc;

// Define methods using the new trait-based API
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

struct AddMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for AddMethod {
    fn method_name(&self) -> &'static str {
        "add"
    }

    async fn call(&self, params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
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
    }
}

struct MultiplyMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for MultiplyMethod {
    fn method_name(&self) -> &'static str {
        "multiply"
    }

    async fn call(&self, params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
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
    }
}

#[tokio::main]
async fn main() {
    // Simplified setup with just metrics and logging
    let observability = observable_setup! {
        service_name: "ash-rpc-server",
        metrics_prefix: "ash_rpc",
    };

    // Extract components
    let logger = observability.logger();
    let metrics = observability.metrics();

    logger.info("Observability stack initialized", &[]);

    // Create method registry with trait-based methods
    let registry = MethodRegistry::new(register_methods![
        PingMethod,
        EchoMethod,
        AddMethod,
        MultiplyMethod,
    ]);

    // Wrap registry in observable processor
    let observable_processor = ObservableProcessor::builder(Arc::new(registry))
        .with_metrics(Arc::clone(&metrics))
        .with_logger(Arc::clone(&logger))
        .build();

    let processor = Arc::new(observable_processor);

    logger.info(
        "Method registry configured",
        &[("methods", &"ping, echo, add, multiply")],
    );

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
        Arc<ash_rpc_contrib::observability::prometheus::PrometheusMetrics>,
    )>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    metrics.connection_opened();

    let response = if request.is_array() {
        let messages: Vec<Message> = serde_json::from_value(request).unwrap_or_default();
        let mut responses = Vec::new();

        for message in messages {
            if let Some(response) = processor.process_message(message).await {
                responses.push(response);
            }
        }

        serde_json::to_value(responses).unwrap()
    } else {
        match serde_json::from_value::<Message>(request) {
            Ok(message) => {
                if let Some(response) = processor.process_message(message).await {
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
