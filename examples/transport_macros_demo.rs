use ash_rpc_core::*;
use std::pin::Pin;
use std::future::Future;

struct PingMethod;

impl JsonRPCMethod for PingMethod {
    fn method_name(&self) -> &'static str {
        "ping"
    }
    
    fn call<'a>(
        &'a self,
        _params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            rpc_success!("pong", id)
        })
    }
}

struct EchoMethod;

impl JsonRPCMethod for EchoMethod {
    fn method_name(&self) -> &'static str {
        "echo"
    }
    
    fn call<'a>(
        &'a self,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            rpc_success!(params.unwrap_or(serde_json::json!(null)), id)
        })
    }
}

#[tokio::main]
async fn main() {
    println!("Transport Macros Demo");
    println!("===================");

    let registry = MethodRegistry::new(register_methods![PingMethod, EchoMethod]);

    println!("✅ Created JSON-RPC registry with methods: ping, echo");

    let test_request = RequestBuilder::new("ping")
        .id(serde_json::json!(1))
        .build();
    let test_message = Message::Request(test_request);

    if let Some(response) = registry.process_message(test_message).await {
        println!(
            "✅ Registry test successful: {}",
            serde_json::to_string(&response).unwrap()
        );
    }

    println!("\n📚 Available Transport Macros:");
    println!("==============================");

    println!("🔧 TCP Server (requires 'tcp' feature):");
    println!("   rpc_tcp_server!(\"127.0.0.1:8080\", registry)");

    println!("\n🚀 TCP Streaming Server (requires 'tcp-stream' feature):");
    println!("   rpc_tcp_stream_server!(\"127.0.0.1:8080\", registry).await");

    println!("\n📡 TCP Streaming Client (requires 'tcp-stream' feature):");
    println!("   let client = rpc_tcp_stream_client!(\"127.0.0.1:8080\").await?;");

    println!("\n🌐 Axum HTTP Router (requires 'axum' feature):");
    println!("   let app = rpc_axum_router!(registry);");
    println!("   let app = rpc_axum_router!(registry, \"/api/rpc\");");

    println!("\n🚀 Axum HTTP Server (requires 'axum' feature):");
    println!("   rpc_axum_server!(\"127.0.0.1:3000\", registry).await");

    println!("\n🔧 Axum RPC Layer (requires 'axum' feature):");
    println!("   let layer = rpc_axum_layer!(registry);");

    println!("\n💡 Usage Examples:");
    println!("=================");

    println!("// TCP Server");
    println!("#[tokio::main]");
    println!("async fn main() -> Result<(), std::io::Error> {{");
    println!("    let registry = MethodRegistry::new()");
    println!("        .register(\"ping\", |_, id| rpc_success!(\"pong\", id));");
    println!("    rpc_tcp_server!(\"127.0.0.1:8080\", registry)");
    println!("}}");

    println!("\n// Axum Server");
    println!("#[tokio::main]");
    println!("async fn main() -> Result<(), Box<dyn std::error::Error>> {{");
    println!("    let registry = MethodRegistry::new()");
    println!("        .register(\"ping\", |_, id| rpc_success!(\"pong\", id));");
    println!("    rpc_axum_server!(\"127.0.0.1:3000\", registry).await?;");
    println!("    Ok(())");
    println!("}}");
}
