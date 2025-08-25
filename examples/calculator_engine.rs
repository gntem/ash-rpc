
use ash_rpc_core::{
    MethodRegistry, Request, Response, rpc_success, rpc_invalid_params, rpc_internal_error, rpc_error
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct MathParams {
    a: f64,
    b: f64,
}

#[derive(Debug, Serialize)]
struct CalculationResult {
    result: f64,
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
        let registry = MethodRegistry::new()
            .register("add", |params, id| {
                let params: MathParams = match params {
                    Some(p) => match serde_json::from_value(p) {
                        Ok(params) => params,
                        Err(_) => return rpc_invalid_params!("Invalid parameters for add method", id),
                    },
                    None => return rpc_invalid_params!("Missing parameters for add method", id),
                };

                let result = CalculationResult {
                    result: params.a + params.b,
                };

                match serde_json::to_value(result) {
                    Ok(result_json) => rpc_success!(result_json, id),
                    Err(_) => rpc_internal_error!("Failed to serialize result", id),
                }
            })
            .register("subtract", |params, id| {
                let params: MathParams = match params {
                    Some(p) => match serde_json::from_value(p) {
                        Ok(params) => params,
                        Err(_) => return rpc_invalid_params!("Invalid parameters for subtract method", id),
                    },
                    None => return rpc_invalid_params!("Missing parameters for subtract method", id),
                };

                let result = CalculationResult {
                    result: params.a - params.b,
                };

                match serde_json::to_value(result) {
                    Ok(result_json) => rpc_success!(result_json, id),
                    Err(_) => rpc_internal_error!("Failed to serialize result", id),
                }
            })
            .register("multiply", |params, id| {
                let params: MathParams = match params {
                    Some(p) => match serde_json::from_value(p) {
                        Ok(params) => params,
                        Err(_) => return rpc_invalid_params!("Invalid parameters for multiply method", id),
                    },
                    None => return rpc_invalid_params!("Missing parameters for multiply method", id),
                };

                let result = CalculationResult {
                    result: params.a * params.b,
                };

                match serde_json::to_value(result) {
                    Ok(result_json) => rpc_success!(result_json, id),
                    Err(_) => rpc_internal_error!("Failed to serialize result", id),
                }
            })
            .register("divide", |params, id| {
                let params: MathParams = match params {
                    Some(p) => match serde_json::from_value(p) {
                        Ok(params) => params,
                        Err(_) => return rpc_invalid_params!("Invalid parameters for divide method", id),
                    },
                    None => return rpc_invalid_params!("Missing parameters for divide method", id),
                };

                if params.b == 0.0 {
                    return rpc_invalid_params!("Division by zero", id);
                }

                let result = CalculationResult {
                    result: params.a / params.b,
                };

                match serde_json::to_value(result) {
                    Ok(result_json) => rpc_success!(result_json, id),
                    Err(_) => rpc_internal_error!("Failed to serialize result", id),
                }
            })
            .register("list_methods", |_params, id| {
                let methods = vec!["add", "subtract", "multiply", "divide", "list_methods"];
                rpc_success!(methods, id)
            });

        Self { registry: Arc::new(registry) }
    }

    pub async fn execute(&self, request: Request) -> Response {
        self.registry.call(&request.method, request.params, request.id)
    }

    pub fn list_methods(&self) -> Vec<String> {
        self.registry.get_methods()
    }

    pub fn has_method(&self, method: &str) -> bool {
        self.registry.has_method(method)
    }

    pub fn render_docs(&mut self) -> serde_json::Value {
        Arc::get_mut(&mut self.registry)
            .map(|registry| registry.render_docs())
            .unwrap_or_else(|| {
                serde_json::json!({
                    "error": "Cannot generate documentation while registry is in use"
                })
            })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("=== Calculator Engine Demo ===");
    
    let mut engine = CalculatorEngine::new();
    
    println!("Engine initialized with methods: {:?}", engine.list_methods());
    println!();

    println!("=== Generated API Documentation ===");
    let docs = engine.render_docs();
    
    if let Some(paths) = docs.get("paths") {
        if let Some(root_path) = paths.get("/") {
            if let Some(post) = root_path.get("post") {
                if let Some(request_body) = post.get("requestBody") {
                    if let Some(content) = request_body.get("content") {
                        if let Some(json_content) = content.get("application/json") {
                            if let Some(schema) = json_content.get("schema") {
                                if let Some(one_of) = schema.get("oneOf") {
                                    if let serde_json::Value::Array(methods) = one_of {
                                        for method in methods {
                                            if let Some(props) = method.get("properties") {
                                                if let Some(method_name) = props.get("method") {
                                                    if let Some(enum_val) = method_name.get("enum") {
                                                        if let serde_json::Value::Array(names) = enum_val {
                                                            if let Some(name) = names.first() {
                                                                println!("Method: {}", name);
                                                                if let Some(params) = props.get("params") {
                                                                    println!("  Parameters: {}", serde_json::to_string_pretty(params).unwrap_or_default());
                                                                }
                                                                println!();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

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
