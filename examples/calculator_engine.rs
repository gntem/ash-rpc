use ash_rpc_core::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;

#[derive(Debug, Deserialize)]
struct MathParams {
    a: f64,
    b: f64,
}

#[derive(Debug, Serialize)]
struct CalculationResult {
    result: f64,
}

struct AddMethod;

impl JsonRPCMethod for AddMethod {
    fn method_name(&self) -> &'static str {
        "add"
    }
    
    fn call<'a>(
        &'a self,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            let params: MathParams = match params {
                Some(p) => match serde_json::from_value(p) {
                    Ok(params) => params,
                    Err(_) => {
                        return rpc_error!(error_codes::INVALID_PARAMS, "Invalid parameters for add method", id);
                    }
                },
                None => return rpc_error!(error_codes::INVALID_PARAMS, "Missing parameters for add method", id),
            };

            let result = CalculationResult {
                result: params.a + params.b,
            };

            match serde_json::to_value(result) {
                Ok(result_json) => rpc_success!(result_json, id),
                Err(_) => rpc_error!(error_codes::INTERNAL_ERROR, "Failed to serialize result", id),
            }
        })
    }
}

struct SubtractMethod;

impl JsonRPCMethod for SubtractMethod {
    fn method_name(&self) -> &'static str {
        "subtract"
    }
    
    fn call<'a>(
        &'a self,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            let params: MathParams = match params {
                Some(p) => match serde_json::from_value(p) {
                    Ok(params) => params,
                    Err(_) => {
                        return rpc_error!(error_codes::INVALID_PARAMS, "Invalid parameters for subtract method", id);
                    }
                },
                None => {
                    return rpc_error!(error_codes::INVALID_PARAMS, "Missing parameters for subtract method", id);
                }
            };

            let result = CalculationResult {
                result: params.a - params.b,
            };

            match serde_json::to_value(result) {
                Ok(result_json) => rpc_success!(result_json, id),
                Err(_) => rpc_error!(error_codes::INTERNAL_ERROR, "Failed to serialize result", id),
            }
        })
    }
}

struct MultiplyMethod;

impl JsonRPCMethod for MultiplyMethod {
    fn method_name(&self) -> &'static str {
        "multiply"
    }
    
    fn call<'a>(
        &'a self,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            let params: MathParams = match params {
                Some(p) => match serde_json::from_value(p) {
                    Ok(params) => params,
                    Err(_) => {
                        return rpc_error!(error_codes::INVALID_PARAMS, "Invalid parameters for multiply method", id);
                    }
                },
                None => {
                    return rpc_error!(error_codes::INVALID_PARAMS, "Missing parameters for multiply method", id);
                }
            };

            let result = CalculationResult {
                result: params.a * params.b,
            };

            match serde_json::to_value(result) {
                Ok(result_json) => rpc_success!(result_json, id),
                Err(_) => rpc_error!(error_codes::INTERNAL_ERROR, "Failed to serialize result", id),
            }
        })
    }
}

struct DivideMethod;

impl JsonRPCMethod for DivideMethod {
    fn method_name(&self) -> &'static str {
        "divide"
    }
    
    fn call<'a>(
        &'a self,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            let params: MathParams = match params {
                Some(p) => match serde_json::from_value(p) {
                    Ok(params) => params,
                    Err(_) => {
                        return rpc_error!(error_codes::INVALID_PARAMS, "Invalid parameters for divide method", id);
                    }
                },
                None => return rpc_error!(error_codes::INVALID_PARAMS, "Missing parameters for divide method", id),
            };

            if params.b == 0.0 {
                return rpc_error!(error_codes::INVALID_PARAMS, "Division by zero", id);
            }

            let result = CalculationResult {
                result: params.a / params.b,
            };

            match serde_json::to_value(result) {
                Ok(result_json) => rpc_success!(result_json, id),
                Err(_) => rpc_error!(error_codes::INTERNAL_ERROR, "Failed to serialize result", id),
            }
        })
    }
}

struct ListMethodsMethod;

impl JsonRPCMethod for ListMethodsMethod {
    fn method_name(&self) -> &'static str {
        "list_methods"
    }
    
    fn call<'a>(
        &'a self,
        _params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            let methods = vec!["add", "subtract", "multiply", "divide", "list_methods"];
            rpc_success!(methods, id)
        })
    }
}

#[derive(Clone)]
pub struct CalculatorEngine {
    registry: Arc<MethodRegistry>,
}

impl Default for CalculatorEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CalculatorEngine {
    pub fn new() -> Self {
        let registry = MethodRegistry::new(register_methods![
            AddMethod,
            SubtractMethod,
            MultiplyMethod,
            DivideMethod,
            ListMethodsMethod
        ]);

        Self {
            registry: Arc::new(registry),
        }
    }

    pub async fn execute(&self, request: Request) -> Response {
        self.registry
            .call(&request.method, request.params, request.id)
            .await
    }

    pub fn list_methods(&self) -> Vec<String> {
        self.registry.get_methods()
    }

    pub fn has_method(&self, method: &str) -> bool {
        self.registry.has_method(method)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("=== Calculator Engine Demo ===");

    let engine = CalculatorEngine::new();

    println!(
        "Engine initialized with methods: {:?}",
        engine.list_methods()
    );
    println!();

    println!("=== Testing Calculator Engine ===");

    let test_requests = vec![
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "add",
            "params": {"a": 10.0, "b": 5.0},
            "id": 1
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "subtract",
            "params": {"a": 10.0, "b": 3.0},
            "id": 2
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "multiply",
            "params": {"a": 4.0, "b": 5.0},
            "id": 3
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "divide",
            "params": {"a": 20.0, "b": 4.0},
            "id": 4
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "divide",
            "params": {"a": 10.0, "b": 0.0},
            "id": 5
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "list_methods",
            "id": 6
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "unknown_method",
            "params": {"a": 1.0, "b": 2.0},
            "id": 7
        }),
    ];

    for test_request in test_requests {
        let request: Request = serde_json::from_value(test_request.clone()).unwrap();
        println!("Request: {}", serde_json::to_string_pretty(&test_request)?);

        let response = engine.execute(request).await;
        println!("Response: {}", serde_json::to_string_pretty(&response)?);
        println!("---");
    }

    Ok(())
}
