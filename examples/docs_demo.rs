use ash_rpc_core::{MethodInfo, MethodRegistry, rpc_error, rpc_invalid_params, rpc_success};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct UserParams {
    name: String,
    age: u32,
}

#[derive(Debug, Serialize)]
struct UserResult {
    id: u32,
    name: String,
    age: u32,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct MathParams {
    a: f64,
    b: f64,
}

#[derive(Debug, Serialize)]
struct MathResult {
    result: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("=== Documentation Generation Demo ===");

    let mut registry = MethodRegistry::new()
        .register_with_info(
            "create_user",
            MethodInfo::new("create_user")
                .with_description("Create a new user account")
                .with_params_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "User's full name",
                            "example": "John Doe"
                        },
                        "age": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 150,
                            "description": "User's age in years",
                            "example": 30
                        }
                    },
                    "required": ["name", "age"]
                }))
                .with_result_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "integer",
                            "description": "Unique user identifier"
                        },
                        "name": {
                            "type": "string",
                            "description": "User's full name"
                        },
                        "age": {
                            "type": "integer",
                            "description": "User's age in years"
                        },
                        "created_at": {
                            "type": "string",
                            "format": "date-time",
                            "description": "Account creation timestamp"
                        }
                    },
                    "required": ["id", "name", "age", "created_at"]
                })),
            |params, id| {
                let params: UserParams = match params {
                    Some(p) => match serde_json::from_value(p) {
                        Ok(params) => params,
                        Err(_) => return rpc_invalid_params!("Invalid user parameters", id),
                    },
                    None => return rpc_invalid_params!("Missing user parameters", id),
                };

                let user = UserResult {
                    id: 123,
                    name: params.name,
                    age: params.age,
                    created_at: "2025-08-20T10:30:00Z".to_string(),
                };

                rpc_success!(user, id)
            },
        )
        .register_with_info(
            "add",
            MethodInfo::new("add")
                .with_description("Add two numbers together")
                .with_params_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "a": {
                            "type": "number",
                            "description": "First number",
                            "example": 10.5
                        },
                        "b": {
                            "type": "number",
                            "description": "Second number",
                            "example": 5.2
                        }
                    },
                    "required": ["a", "b"]
                }))
                .with_result_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "result": {
                            "type": "number",
                            "description": "Sum of the two numbers"
                        }
                    },
                    "required": ["result"]
                })),
            |params, id| {
                let params: MathParams = match params {
                    Some(p) => match serde_json::from_value(p) {
                        Ok(params) => params,
                        Err(_) => return rpc_invalid_params!("Invalid math parameters", id),
                    },
                    None => return rpc_invalid_params!("Missing math parameters", id),
                };

                let result = MathResult {
                    result: params.a + params.b,
                };

                rpc_success!(result, id)
            },
        )
        .register("ping", |_params, id| rpc_success!("pong", id));

    println!("Registry created with {} methods", registry.method_count());
    println!("Available methods: {:?}", registry.get_methods());
    println!();

    println!("Generating OpenAPI documentation...");
    let docs = registry.render_docs();

    println!("Generated OpenAPI Documentation:");
    println!("{}", serde_json::to_string_pretty(&docs)?);

    Ok(())
}
