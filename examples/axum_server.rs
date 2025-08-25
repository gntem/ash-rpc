#[cfg(feature = "axum")]
mod example {
    use ash_rpc_core::{
        transport::axum::AxumRpcLayer,
        MessageProcessor, Message, Response, ResponseBuilder
    };
    use axum::Router;

    struct CalculatorProcessor;

    impl MessageProcessor for CalculatorProcessor {
        fn process_message(&self, message: Message) -> Option<Response> {
            match message {
                Message::Request(req) => {
                    let result = match req.method.as_str() {
                        "add" => {
                            if let Some(params) = req.params {
                                if let (Some(a), Some(b)) = (params.get("a"), params.get("b")) {
                                    if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
                                        serde_json::json!(a + b)
                                    } else {
                                        return Some(ResponseBuilder::new()
                                            .error(ash_rpc_core::ErrorBuilder::new(
                                                -32602,
                                                "Invalid parameters: expected numbers"
                                            ).build())
                                            .id(req.id)
                                            .build());
                                    }
                                } else {
                                    return Some(ResponseBuilder::new()
                                        .error(ash_rpc_core::ErrorBuilder::new(
                                            -32602,
                                            "Missing parameters: a and b required"
                                        ).build())
                                        .id(req.id)
                                        .build());
                                }
                            } else {
                                return Some(ResponseBuilder::new()
                                    .error(ash_rpc_core::ErrorBuilder::new(
                                        -32602,
                                        "Missing parameters"
                                    ).build())
                                    .id(req.id)
                                    .build());
                            }
                        }
                        "multiply" => {
                            if let Some(params) = req.params {
                                if let (Some(a), Some(b)) = (params.get("a"), params.get("b")) {
                                    if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
                                        serde_json::json!(a * b)
                                    } else {
                                        return Some(ResponseBuilder::new()
                                            .error(ash_rpc_core::ErrorBuilder::new(
                                                -32602,
                                                "Invalid parameters: expected numbers"
                                            ).build())
                                            .id(req.id)
                                            .build());
                                    }
                                } else {
                                    return Some(ResponseBuilder::new()
                                        .error(ash_rpc_core::ErrorBuilder::new(
                                            -32602,
                                            "Missing parameters: a and b required"
                                        ).build())
                                        .id(req.id)
                                        .build());
                                }
                            } else {
                                return Some(ResponseBuilder::new()
                                    .error(ash_rpc_core::ErrorBuilder::new(
                                        -32602,
                                        "Missing parameters"
                                    ).build())
                                    .id(req.id)
                                    .build());
                            }
                        }
                        _ => {
                            return Some(ResponseBuilder::new()
                                .error(ash_rpc_core::ErrorBuilder::new(
                                    -32601,
                                    "Method not found"
                                ).build())
                                .id(req.id)
                                .build());
                        }
                    };

                    Some(ResponseBuilder::new()
                        .success(result)
                        .id(req.id)
                        .build())
                }
                Message::Notification(_) => None,
                Message::Response(_) => None,
            }
        }
    }

    pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
        let rpc_layer = AxumRpcLayer::builder()
            .processor(CalculatorProcessor)
            .path("/api/rpc")
            .build()?;

        let app = Router::new()
            .merge(rpc_layer.into_router());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
        println!("Axum RPC server listening on http://127.0.0.1:3000/api/rpc");
        
        axum::serve(listener, app).await?;
        Ok(())
    }
}

#[cfg(feature = "axum")]
fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(example::run_server())
        .unwrap();
}

#[cfg(not(feature = "axum"))]
fn main() {
    println!("This example requires the 'axum' feature to be enabled.");
    println!("Run with: cargo run --example axum_server --features axum");
}
