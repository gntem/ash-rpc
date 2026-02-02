//! Example demonstrating the healthcheck functionality from ash-rpc-contrib

use ash_rpc::contrib::healthcheck::HealthcheckMethod;
use ash_rpc::*;

#[tokio::main]
async fn main() {
    // Create a registry with healthcheck method
    let healthcheck = HealthcheckMethod::new()
        .service_name("example-service")
        .with_version("1.0.0");

    let registry = MethodRegistry::new(register_methods![healthcheck]);

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
