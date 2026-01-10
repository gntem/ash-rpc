use ash_rpc_core::*;

// Example showing rpc_validate! macro for validation
rpc_method!(
    CreateUserMethod,
    "create_user",
    |params: Option<serde_json::Value>, id| {
        #[derive(serde::Deserialize)]
        struct UserData {
            name: String,
            age: u32,
            email: String,
        }

        let user: UserData = rpc_params!(params, id => UserData);

        // Use rpc_validate! for input validation
        rpc_validate!(!user.name.is_empty(), "Name cannot be empty", id);
        rpc_validate!(user.age >= 18, "User must be at least 18 years old", id);
        rpc_validate!(user.email.contains('@'), "Invalid email format", id);

        let response = serde_json::json!({
            "id": 123,
            "name": user.name,
            "age": user.age,
            "email": user.email,
            "status": "active"
        });

        rpc_success!(response, id)
    }
);

rpc_method!(
    GetServerInfoMethod,
    "get_server_info",
    |_params: Option<serde_json::Value>, id| {
        let info = serde_json::json!({
            "name": "ash-rpc-server",
            "version": "2.1.0",
            "uptime": "5 minutes",
            "methods": ["ping", "create_user", "get_server_info"]
        });

        rpc_success!(info, id)
    }
);

rpc_method!(PingMethod, "ping", |_params: Option<serde_json::Value>,
                                 id| {
    rpc_success!("pong", id)
});

#[tokio::main]
async fn main() {
    println!("=== Advanced Macros Demo ===");

    // Using rpc_registry_with_methods! macro
    let registry = rpc_registry_with_methods![CreateUserMethod, GetServerInfoMethod, PingMethod];

    // Test multiple requests using rpc_call_request! macro
    let requests = vec![
        rpc_call_request!("ping", 1),
        rpc_call_request!("get_server_info", 2),
        rpc_call_request!(
            "create_user",
            serde_json::json!({
                "name": "Alice",
                "age": 25,
                "email": "alice@example.com"
            }),
            3
        ),
        // This should fail validation
        rpc_call_request!(
            "create_user",
            serde_json::json!({
                "name": "",
                "age": 16,
                "email": "invalid-email"
            }),
            4
        ),
    ];

    // Process all requests
    for request in requests {
        let response = registry
            .call(&request.method, request.params, request.id)
            .await;
        println!(
            "Method: {} -> {}",
            request.method,
            if response.is_error() {
                "ERROR"
            } else {
                "SUCCESS"
            }
        );
        println!("{}\n", serde_json::to_string_pretty(&response).unwrap());

        // Demonstrate rpc_extract! with different types
        if response.is_success() {
            match request.method.as_str() {
                "ping" => {
                    let pong: String = rpc_extract!(response => String);
                    println!("Extracted ping result: '{}'\n", pong);
                }
                "get_server_info" => {
                    let raw_value = rpc_extract!(response);
                    println!("Raw server info: {}\n", raw_value);
                }
                "create_user" => {
                    let raw_user = rpc_extract!(response);
                    if let Some(user_id) = raw_user.get("id") {
                        println!("Created user with ID: {}\n", user_id);
                    }
                }
                _ => {}
            }
        }
    }

    println!("Registry has {} methods", registry.method_count());
    println!("Available methods: {:?}", registry.get_methods());
}
