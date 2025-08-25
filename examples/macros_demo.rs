use ash_rpc_core::*;

fn main() {
    
    let request = rpc_request!("add", [5, 3], 1);
    println!("Request: {}", serde_json::to_string_pretty(&request).unwrap());
    
    let log_data = serde_json::json!({"level": "info", "message": "Hello World"});
    let notification = rpc_notification!("log", log_data);
    println!("Notification: {}", serde_json::to_string_pretty(&notification).unwrap());
    
    let success_response = rpc_success!(8, Some(serde_json::json!(1)));
    println!("Success Response: {}", serde_json::to_string_pretty(&success_response).unwrap());
    
    let error_response = rpc_invalid_params!("Expected array of two numbers", Some(serde_json::json!(1)));
    println!("Error Response: {}", serde_json::to_string_pretty(&error_response).unwrap());
}
