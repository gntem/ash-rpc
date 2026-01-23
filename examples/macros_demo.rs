use ash_rpc_core::*;

fn main() {
    println!("=== Basic Macros Demo ===\n");

    // Request macro
    let request = RequestBuilder::new("add")
        .params(serde_json::json!([5, 3]))
        .id(serde_json::json!(1))
        .build();
    println!(
        "Request: {}",
        serde_json::to_string_pretty(&request).unwrap()
    );

    // Notification macro
    let log_data = serde_json::json!({"level": "info", "message": "Hello World"});
    let notification = NotificationBuilder::new("log").params(log_data).build();
    println!(
        "Notification: {}",
        serde_json::to_string_pretty(&notification).unwrap()
    );

    // Success response macro
    let success_response = rpc_success!(8, Some(serde_json::json!(1)));
    println!(
        "Success Response: {}",
        serde_json::to_string_pretty(&success_response).unwrap()
    );

    // Error response macro
    let error_response = rpc_error!(
        error_codes::INVALID_PARAMS,
        "Expected array of two numbers",
        Some(serde_json::json!(1))
    );
    println!(
        "Error Response: {}",
        serde_json::to_string_pretty(&error_response).unwrap()
    );

    println!("\n=== Available Transport Macros ===");
    #[cfg(feature = "tcp")]
    println!("rpc_tcp_server! - Create a TCP server");

    #[cfg(feature = "tcp-stream")]
    println!("rpc_tcp_stream_server! - Create a TCP streaming server");

    #[cfg(feature = "tcp-stream")]
    println!("rpc_tcp_stream_client! - Create a TCP streaming client");

    // Note: Axum and WebSocket features are not available in this core version
    println!("rpc_axum_* macros - Requires contrib package with axum feature");
    println!("rpc_websocket_* macros - Requires contrib package with websocket feature");

    println!("\n=== Available Stateful Macros ===");
    #[cfg(feature = "stateful")]
    println!("rpc_stateful_processor! - Create a stateful processor");

    #[cfg(feature = "stateful")]
    println!("rpc_stateful_registry! - Create a stateful method registry");

    #[cfg(feature = "stateful")]
    println!("rpc_stateful_builder! - Create a stateful processor with builder");

    println!("\nSee transport_macros_demo.rs for usage examples!");
}
