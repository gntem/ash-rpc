use ash_rpc_core::*;

fn main() {
    let registry = MethodRegistry::new()
        .register("add", |params, id| {
            if let Some(params) = params {
                if let Ok(numbers) = serde_json::from_value::<[i32; 2]>(params) {
                    let result = numbers[0] + numbers[1];
                    ResponseBuilder::new()
                        .success(serde_json::json!(result))
                        .id(id)
                        .build()
                } else {
                    ResponseBuilder::new()
                        .error(
                            ErrorBuilder::new(error_codes::INVALID_PARAMS, "Invalid parameters")
                                .build(),
                        )
                        .id(id)
                        .build()
                }
            } else {
                ResponseBuilder::new()
                    .error(
                        ErrorBuilder::new(error_codes::INVALID_PARAMS, "Missing parameters")
                            .build(),
                    )
                    .id(id)
                    .build()
            }
        })
        .register("subtract", |params, id| {
            if let Some(params) = params {
                if let Ok(numbers) = serde_json::from_value::<[i32; 2]>(params) {
                    let result = numbers[0] - numbers[1];
                    ResponseBuilder::new()
                        .success(serde_json::json!(result))
                        .id(id)
                        .build()
                } else {
                    ResponseBuilder::new()
                        .error(
                            ErrorBuilder::new(error_codes::INVALID_PARAMS, "Invalid parameters")
                                .build(),
                        )
                        .id(id)
                        .build()
                }
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

    let request = RequestBuilder::new("add")
        .params(serde_json::json!([5, 3]))
        .id(serde_json::json!(1))
        .build();

    let message = Message::Request(request);

    if let Some(response) = registry.process_message(message) {
        println!(
            "Response: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );
    }

    let notification = NotificationBuilder::new("log")
        .params(serde_json::json!({"level": "info", "message": "Hello World"}))
        .build();

    let message = Message::Notification(notification);
    registry.process_message(message);
}
