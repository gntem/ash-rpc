use ash_rpc_core::transport::tcp::TcpServer;
use ash_rpc_core::*;

fn main() -> Result<(), std::io::Error> {
    let logger = StdoutLogger;
    logger.info("Starting JSON RPC TCP Server", &[("addr", &"127.0.0.1:8080")]);

    let registry = MethodRegistry::new()
        .register("ping", |_params, id| rpc_success!("pong", id))
        .register("echo", |params, id| {
            rpc_success!(params.unwrap_or(serde_json::json!(null)), id)
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
        })
        .register("log", |params, _id| {
            if let Some(params) = params {
                println!("LOG: {}", params);
            }
            rpc_success!("logged", None)
        });

    let server = TcpServer::builder("127.0.0.1:8080")
        .processor(registry)
        .logger(logger)
        .build()?;

    logger.info("Available methods: ping, echo, add, log", &[]);
    logger.info("Example requests:", &[]);
    println!(r#"  {{"jsonrpc":"2.0","method":"ping","id":1}}"#);
    println!(r#"  {{"jsonrpc":"2.0","method":"echo","params":"hello","id":2}}"#);
    println!(r#"  {{"jsonrpc":"2.0","method":"add","params":[5,3],"id":3}}"#);
    println!(r#"  {{"jsonrpc":"2.0","method":"log","params":"test message"}}"#);
    println!();

    server.run()
}
