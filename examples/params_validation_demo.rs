use ash_rpc::*;

// Example showing rpc_params! macro for parameter validation
rpc_method!(
    AddNumbersMethod,
    "add_numbers",
    |params: Option<serde_json::Value>, id| {
        // Using rpc_params! to validate and extract required parameters
        let numbers: Vec<i32> = rpc_params!(params, id => Vec<i32>);
        let sum = numbers.iter().sum::<i32>();
        rpc_success!(sum, id)
    }
);

// Example with optional parameters
rpc_method!(
    GreetMethod,
    "greet",
    |params: Option<serde_json::Value>, id| {
        let name: Option<String> = rpc_params!(params, id => Option<String>);
        let greeting = match name {
            Some(n) => format!("Hello, {}!", n),
            None => "Hello, World!".to_string(),
        };
        rpc_success!(greeting, id)
    }
);

// Example with rpc_try! error handling
rpc_method!(
    SafeDivideMethod,
    "safe_divide",
    |params: Option<serde_json::Value>, id| {
        let [dividend, divisor]: [f64; 2] = rpc_params!(params, id => [f64; 2]);

        let result: Result<f64, &str> = if divisor != 0.0 {
            Ok(dividend / divisor)
        } else {
            Err("Cannot divide by zero")
        };

        // Use rpc_try! to convert Result to Response
        rpc_try!(result, id)
    }
);

#[tokio::main]
async fn main() {
    println!("=== Parameter Validation Demo ===");

    let registry = MethodRegistry::new(register_methods![
        AddNumbersMethod,
        GreetMethod,
        SafeDivideMethod
    ]);

    // Test add_numbers with valid params
    let request = RequestBuilder::new("add_numbers")
        .params(serde_json::json!([1, 2, 3, 4, 5]))
        .id(serde_json::json!(1))
        .build();
    let response = registry
        .call("add_numbers", request.params, request.id)
        .await;
    println!(
        "Add numbers response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    // Test greet with name
    let request = RequestBuilder::new("greet")
        .params(serde_json::json!("Alice"))
        .id(serde_json::json!(2))
        .build();
    let response = registry.call("greet", request.params, request.id).await;
    println!(
        "Greet with name: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    // Test greet without name (optional param)
    let request = RequestBuilder::new("greet")
        .id(serde_json::json!(3))
        .build();
    let response = registry.call("greet", request.params, request.id).await;
    println!(
        "Greet without name: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    // Extract results using rpc_extract! macro
    let greeting: String = rpc_extract!(response => String);
    println!("Extracted greeting: '{}'", greeting);

    // Test safe_divide with valid params
    let request = RequestBuilder::new("safe_divide")
        .params(serde_json::json!([10.0, 2.0]))
        .id(serde_json::json!(4))
        .build();
    let response = registry
        .call("safe_divide", request.params, request.id)
        .await;
    println!(
        "Safe divide response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    // Test safe_divide with zero divisor (error case)
    let request = RequestBuilder::new("safe_divide")
        .params(serde_json::json!([10.0, 0.0]))
        .id(serde_json::json!(5))
        .build();
    let response = registry
        .call("safe_divide", request.params, request.id)
        .await;
    println!(
        "Safe divide by zero: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );
}
