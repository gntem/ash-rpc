use ash_rpc_core::{
    error_codes, ErrorBuilder, Handler, Message, MessageProcessor, MethodRegistry, Request,
    ResponseBuilder,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = MethodRegistry::new()
        .register("ping", |_params, id| {
            ResponseBuilder::new()
                .success(serde_json::json!("pong"))
                .id(id)
                .build()
        })
        .register("echo", |params, id| {
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
        });

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

    let request = Request::new("ping").with_id(serde_json::json!(1));
    let message = Message::Request(request);

    println!("Message is request: {}", message.is_request());
    println!("Message method: {:?}", message.method());
    println!("Message ID: {:?}", message.id());

    if let Some(response) = registry.process_message(message) {
        println!("Response is success: {}", response.is_success());
        println!("Response result: {:?}", response.result());
        println!("Response: {}", serde_json::to_string_pretty(&response)?);
    }

    let batch = vec![
        Message::Request(Request::new("ping").with_id(serde_json::json!(1))),
        Message::Request(
            Request::new("echo")
                .with_params(serde_json::json!({"message": "hello"}))
                .with_id(serde_json::json!(2)),
        ),
    ];

    let batch_responses = registry.process_batch(batch);
    println!("Batch responses count: {}", batch_responses.len());

    Ok(())
}
