#[cfg(feature = "axum")]
mod example {
    use ash_rpc::{
        ErrorBuilder, MethodRegistry, ResponseBuilder, transport::axum::AxumRpcLayer,
    };
    use axum::Router;

    pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
        let registry = MethodRegistry::new()
            .register("add", |params, id| {
                if let Some(params) = params
                    && let (Some(a), Some(b)) = (params.get("a"), params.get("b"))
                    && let (Some(a), Some(b)) = (a.as_f64(), b.as_f64())
                {
                    return ResponseBuilder::new()
                        .success(serde_json::json!(a + b))
                        .id(id)
                        .build();
                }
                ResponseBuilder::new()
                    .error(ErrorBuilder::new(-32602, "Invalid parameters").build())
                    .id(id)
                    .build()
            })
            .register("subtract", |params, id| {
                if let Some(params) = params
                    && let (Some(a), Some(b)) = (params.get("a"), params.get("b"))
                    && let (Some(a), Some(b)) = (a.as_f64(), b.as_f64())
                {
                    return ResponseBuilder::new()
                        .success(serde_json::json!(a - b))
                        .id(id)
                        .build();
                }
                ResponseBuilder::new()
                    .error(ErrorBuilder::new(-32602, "Invalid parameters").build())
                    .id(id)
                    .build()
            });

        let rpc_layer = AxumRpcLayer::builder().processor(registry).build()?;

        let app = Router::new().merge(rpc_layer.into_router());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;
        println!("Simple Axum RPC server listening on http://127.0.0.1:3001/rpc");

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
    println!("Run with: cargo run --example axum_simple --features axum");
}
