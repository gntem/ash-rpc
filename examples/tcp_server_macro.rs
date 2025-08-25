use ash_rpc_core::*;

fn main() -> Result<(), std::io::Error> {
    println!("Starting TCP JSON-RPC Server with macro on 127.0.0.1:8080");
    
    let registry = MethodRegistry::new()
        .register("ping", |_params, id| {
            rpc_success!("pong", id)
        })
        .register("add", |params, id| {
            if let Some(params) = params {
                if let Ok(numbers) = serde_json::from_value::<[f64; 2]>(params) {
                    let result = numbers[0] + numbers[1];
                    rpc_success!(result, id)
                } else {
                    rpc_invalid_params!("Expected array of two numbers", id)
                }
            } else {
                rpc_invalid_params!("Missing parameters", id)
            }
        });

    println!("Available methods: ping, add");
    println!("Example requests:");
    println!(r#"  {{"jsonrpc":"2.0","method":"ping","id":1}}"#);
    println!(r#"  {{"jsonrpc":"2.0","method":"add","params":[5,3],"id":2}}"#);
    println!();

    rpc_tcp_server!("127.0.0.1:8080", registry)
}
