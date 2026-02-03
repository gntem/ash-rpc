use ash_rpc::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting TCP Streaming JSON-RPC Server with macro on 127.0.0.1:8080");

    let registry = MethodRegistry::new()
        .register("ping", |_params, id| rpc_success!("pong", id))
        .register("echo", |params, id| {
            rpc_success!(params.unwrap_or(serde_json::json!(null)), id)
        })
        .register("time", |_params, id| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            rpc_success!(now, id)
        });

    println!("Available methods: ping, echo, time");
    println!("Example requests:");
    println!(r#"  {{"jsonrpc":"2.0","method":"ping","id":1}}"#);
    println!(r#"  {{"jsonrpc":"2.0","method":"echo","params":"hello","id":2}}"#);
    println!(r#"  {{"jsonrpc":"2.0","method":"time","id":3}}"#);
    println!();

    rpc_tcp_stream_server!("127.0.0.1:8080", registry).await
}
