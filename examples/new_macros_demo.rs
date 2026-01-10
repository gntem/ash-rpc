use ash_rpc_core::*;

// Define methods using the new rpc_method! macro
rpc_method!(PingMethod, "ping", |_params, id| {
    rpc_success!("pong", id)
});

rpc_method!(EchoMethod, "echo", |params: Option<serde_json::Value>,
                                 id| {
    rpc_success!(params.unwrap_or(serde_json::json!(null)), id)
});

rpc_method!(AddMethod, "add", |params: Option<serde_json::Value>, id| {
    let numbers = rpc_params!(params, id => Vec<i32>);
    rpc_success!(numbers.iter().sum::<i32>(), id)
});

rpc_method!(
    DivideMethod,
    "divide",
    |params: Option<serde_json::Value>, id| {
        let [a, b]: [f64; 2] = rpc_params!(params, id => [f64; 2]);
        let result = if b != 0.0 {
            Ok(a / b)
        } else {
            Err("Division by zero")
        };
        rpc_try!(result, id)
    }
);

#[tokio::main]
async fn main() {
    println!("=== New Macros Demo ===");

    let registry = MethodRegistry::new(register_methods![
        PingMethod,
        EchoMethod,
        AddMethod,
        DivideMethod
    ]);

    // Test ping
    let request = RequestBuilder::new("ping").id(serde_json::json!(1)).build();
    let response = registry.call("ping", request.params, request.id).await;
    println!(
        "Ping response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    // Test add with rpc_params validation
    let request = RequestBuilder::new("add")
        .params(serde_json::json!([5, 10, 15]))
        .id(serde_json::json!(2))
        .build();
    let response = registry.call("add", request.params, request.id).await;
    println!(
        "Add response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    // Test divide with rpc_try error handling
    let request = RequestBuilder::new("divide")
        .params(serde_json::json!([10.0, 2.0]))
        .id(serde_json::json!(3))
        .build();
    let response = registry.call("divide", request.params, request.id).await;
    println!(
        "Divide response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    // Extract result using rpc_extract macro
    let result: f64 = rpc_extract!(response => f64);
    println!("Extracted result: {}", result);

    // Test divide by zero
    let request = RequestBuilder::new("divide")
        .params(serde_json::json!([10.0, 0.0]))
        .id(serde_json::json!(4))
        .build();
    let response = registry.call("divide", request.params, request.id).await;
    println!(
        "Divide by zero response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );
}
