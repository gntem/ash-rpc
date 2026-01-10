//! Example demonstrating a simple healthcheck functionality

use ash_rpc_core::*;
use std::pin::Pin;
use std::future::Future;

struct HealthcheckMethod;

impl JsonRPCMethod for HealthcheckMethod {
    fn method_name(&self) -> &'static str {
        "healthcheck"
    }
    
    fn call<'a>(
        &'a self,
        _params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            let health_status = serde_json::json!({
                "status": "healthy",
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                "service": "ash-rpc-example"
            });
            rpc_success!(health_status, id)
        })
    }
}

#[tokio::main]
async fn main() {
    // Create a registry and register the healthcheck method
    let registry = MethodRegistry::new(register_methods![HealthcheckMethod]);

    // Create a healthcheck request
    let request = RequestBuilder::new("healthcheck")
        .id(serde_json::json!(1))
        .build();

    // Process the request
    let message = Message::Request(request);
    if let Some(response) = registry.process_message(message).await {
        println!(
            "Healthcheck response: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );
    }

    // Test with parameters (they should be ignored)
    let request_with_params = RequestBuilder::new("healthcheck")
        .params(serde_json::json!({"service": "api", "version": "1.0"}))
        .id(serde_json::json!(2))
        .build();

    let message_with_params = Message::Request(request_with_params);
    if let Some(response) = registry.process_message(message_with_params).await {
        println!(
            "Healthcheck with params response: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );
    }
}
