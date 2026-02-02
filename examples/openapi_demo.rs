use ash_rpc::*;

// Example method implementations with OpenAPI documentation
struct HelloMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for HelloMethod {
    fn method_name(&self) -> &'static str {
        "hello"
    }

    async fn call(&self, params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        if let Some(params) = params {
            if let Ok(name) = serde_json::from_value::<String>(params["name"].clone()) {
                rpc_success!(format!("Hello, {}!", name), id)
            } else {
                rpc_invalid_params!("Expected 'name' parameter", id)
            }
        } else {
            rpc_success!("Hello, World!", id)
        }
    }

    fn openapi_components(&self) -> OpenApiMethodSpec {
        OpenApiMethodSpec::new("hello")
            .with_summary("Say hello to someone")
            .with_description("Greets the person with the provided name, or says 'Hello, World!' if no name is provided")
            .with_parameters(serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the person to greet"
                    }
                },
                "required": []
            }))
            .with_result(serde_json::json!({
                "type": "string",
                "description": "Greeting message"
            }))
            .with_tag("greetings")
            .with_example(
                OpenApiExample::new("basic_hello")
                    .with_summary("Basic hello example")
                    .with_params(serde_json::json!({"name": "Alice"}))
                    .with_result(serde_json::json!("Hello, Alice!"))
            )
            .with_example(
                OpenApiExample::new("no_params")
                    .with_summary("Hello without parameters")
                    .with_result(serde_json::json!("Hello, World!"))
            )
    }
}

struct GoodbyeMethod;

#[async_trait::async_trait]
impl JsonRPCMethod for GoodbyeMethod {
    fn method_name(&self) -> &'static str {
        "goodbye"
    }

    async fn call(&self, params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
        if let Some(params) = params {
            if let Ok(name) = serde_json::from_value::<String>(params["name"].clone()) {
                rpc_success!(format!("Goodbye, {}!", name), id)
            } else {
                rpc_invalid_params!("Expected 'name' parameter", id)
            }
        } else {
            rpc_success!("Goodbye, World!", id)
        }
    }

    fn openapi_components(&self) -> OpenApiMethodSpec {
        OpenApiMethodSpec::new("goodbye")
            .with_summary("Say goodbye to someone")
            .with_description("Says goodbye to the person with the provided name")
            .with_parameters(serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the person to say goodbye to"
                    }
                },
                "required": ["name"]
            }))
            .with_result(serde_json::json!({
                "type": "string",
                "description": "Goodbye message"
            }))
            .with_tag("greetings")
            .with_error(
                OpenApiError::new(-32602, "Invalid params")
                    .with_description("Required 'name' parameter is missing or invalid"),
            )
            .with_example(
                OpenApiExample::new("basic_goodbye")
                    .with_summary("Basic goodbye example")
                    .with_params(serde_json::json!({"name": "Bob"}))
                    .with_result(serde_json::json!("Goodbye, Bob!")),
            )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("=== OpenAPI Documentation Generation Demo ===\n");

    // Create registry with methods
    let registry = MethodRegistry::new(register_methods![HelloMethod, GoodbyeMethod,]);

    // Generate OpenAPI specification
    let openapi_spec = registry.generate_openapi_spec_with_info(
        "Greeting Service API",
        "1.0.0",
        Some("A simple greeting service demonstrating OpenAPI documentation generation"),
        vec![
            OpenApiServer::new("http://localhost:3000/rpc")
                .with_description("Local development server"),
            OpenApiServer::new("https://api.example.com/rpc").with_description("Production server"),
        ],
    );

    // Export as JSON
    let openapi_json = serde_json::to_string_pretty(&openapi_spec)?;
    println!("Generated OpenAPI Specification:");
    println!("{}", openapi_json);

    println!("\n=== Method Registry Info ===");
    println!("Total methods: {}", registry.method_count());
    println!("Registered methods: {:?}", registry.get_methods());

    // Test some method calls
    println!("\n=== Testing Method Calls ===");

    let hello_response = registry
        .call(
            "hello",
            Some(serde_json::json!({"name": "OpenAPI"})),
            Some(serde_json::json!(1)),
        )
        .await;
    println!(
        "Hello call result: {}",
        serde_json::to_string_pretty(&hello_response)?
    );

    let goodbye_response = registry
        .call(
            "goodbye",
            Some(serde_json::json!({"name": "OpenAPI"})),
            Some(serde_json::json!(2)),
        )
        .await;
    println!(
        "Goodbye call result: {}",
        serde_json::to_string_pretty(&goodbye_response)?
    );

    Ok(())
}
