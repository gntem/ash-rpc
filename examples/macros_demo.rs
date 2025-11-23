use ash_rpc_core::*;

fn main() {
    println!("=== Basic Macros Demo ===\n");

    // Request macro
    let request = rpc_request!("add", [5, 3], 1);
    println!(
        "Request: {}",
        serde_json::to_string_pretty(&request).unwrap()
    );

    // Notification macro
    let log_data = serde_json::json!({"level": "info", "message": "Hello World"});
    let notification = rpc_notification!("log", log_data);
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
    let error_response =
        rpc_invalid_params!("Expected array of two numbers", Some(serde_json::json!(1)));
    println!(
        "Error Response: {}",
        serde_json::to_string_pretty(&error_response).unwrap()
    );

    println!("\n=== Available Transport Macros ===");
    #[cfg(feature = "tcp")]
    println!("✓ rpc_tcp_server! - Create a TCP server");

    #[cfg(feature = "tcp-stream")]
    println!("✓ rpc_tcp_stream_server! - Create a TCP streaming server");
    
    #[cfg(feature = "tcp-stream")]
    println!("✓ rpc_tcp_stream_client! - Create a TCP streaming client");

    #[cfg(feature = "axum")]
    println!("✓ rpc_axum_router! - Create an Axum router");
    
    #[cfg(feature = "axum")]
    println!("✓ rpc_axum_server! - Create and run an Axum server");
    
    #[cfg(feature = "axum")]
    println!("✓ rpc_axum_layer! - Create an Axum middleware layer");

    #[cfg(feature = "websocket")]
    println!("✓ rpc_websocket_server! - Create a WebSocket server");

    println!("\n=== Available Stateful Macros ===");
    #[cfg(feature = "stateful")]
    println!("✓ rpc_stateful_processor! - Create a stateful processor");
    
    #[cfg(feature = "stateful")]
    println!("✓ rpc_stateful_registry! - Create a stateful method registry");
    
    #[cfg(feature = "stateful")]
    println!("✓ rpc_stateful_builder! - Create a stateful processor with builder");

    println!("\n✨ See transport_macros_demo.rs and stateful_websocket_macro.rs for usage examples!");
}
