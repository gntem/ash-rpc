use ash_rpc_core::*;

struct AddMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for AddMethod {
    fn method_name(&self) -> &'static str {
        "add"
    }
    
    async fn call(
        &self,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Response {
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
    }
}

struct SubtractMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for SubtractMethod {
    fn method_name(&self) -> &'static str {
        "subtract"
    }
    
    async fn call(
        &self,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Response {
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
    }
}

#[tokio::main]
async fn main() {
    let registry = MethodRegistry::new(register_methods![AddMethod, SubtractMethod]);

    let request = RequestBuilder::new("add")
        .params(serde_json::json!([5, 3]))
        .id(serde_json::json!(1))
        .build();

    let message = Message::Request(request);

    if let Some(response) = registry.process_message(message).await {
        println!(
            "Response: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );
    }

    let notification = NotificationBuilder::new("log")
        .params(serde_json::json!({"level": "info", "message": "Hello World"}))
        .build();

    let message = Message::Notification(notification);
    registry.process_message(message).await;
}
