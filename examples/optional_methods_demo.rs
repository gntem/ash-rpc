use ash_rpc_core::*;

struct PingMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for PingMethod {
    fn method_name(&self) -> &'static str {
        "ping"
    }
    
    async fn call(
        &self,
        _params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Response {
        ResponseBuilder::new()
            .success(serde_json::json!("pong"))
            .id(id)
            .build()
    }
}

struct EchoMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for EchoMethod {
    fn method_name(&self) -> &'static str {
        "echo"
    }
    
    async fn call(
        &self,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Response {
        if let Some(params) = params {
            ResponseBuilder::new().success(params).id(id).build()
        } else {
            ResponseBuilder::new()
                .error(
                    ErrorBuilder::new(error_codes::INVALID_PARAMS, "Missing parameters")
                        .build(),
                )
                .id(id)
                .build()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = MethodRegistry::new(register_methods![PingMethod, EchoMethod]);

    println!("Registry has {} methods", registry.method_count());
    println!("Supported methods: {:?}", registry.get_supported_methods());
    println!("Has ping method: {}", registry.has_method("ping"));
    println!("Has unknown method: {}", registry.has_method("unknown"));

    let capabilities = registry.get_capabilities();
    println!("Supports batching: {}", capabilities.supports_batch);
    println!(
        "Supports notifications: {}",
        capabilities.supports_notifications
    );
    println!("Max batch size: {:?}", capabilities.max_batch_size);

    let request = RequestBuilder::new("ping")
        .id(serde_json::json!(1))
        .build();
    let message = Message::Request(request);

    println!("Message is request: {}", message.is_request());
    println!("Message method: {:?}", message.method());
    println!("Message ID: {:?}", message.id());

    if let Some(response) = registry.process_message(message).await {
        println!("Response is success: {}", response.is_success());
        println!("Response result: {:?}", response.result());
        println!("Response: {}", serde_json::to_string_pretty(&response)?);
    }

    let batch = vec![
        Message::Request(RequestBuilder::new("ping").id(serde_json::json!(1)).build()),
        Message::Request(
            RequestBuilder::new("echo")
                .params(serde_json::json!({"message": "hello"}))
                .id(serde_json::json!(2))
                .build()
        ),
    ];

    let batch_responses = registry.process_batch(batch).await;
    println!("Batch responses count: {}", batch_responses.len());

    Ok(())
}
