#[cfg(feature = "tcp-stream")]
mod example {
    use ash_rpc::{
        Message, MessageProcessor, Response, ResponseBuilder,
        transport::tcp_stream::TcpStreamServer,
    };

    struct EchoProcessor;

    impl MessageProcessor for EchoProcessor {
        fn process_message(&self, message: Message) -> Option<Response> {
            match message {
                Message::Request(req) => Some(
                    ResponseBuilder::new()
                        .success(serde_json::json!({
                            "echo": req.method,
                            "params": req.params
                        }))
                        .id(req.id)
                        .build(),
                ),
                Message::Notification(_) => None,
                Message::Response(_) => None,
            }
        }
    }

    pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
        let server = TcpStreamServer::builder("127.0.0.1:3030")
            .processor(EchoProcessor)
            .build()?;

        println!("Starting TCP Stream server on 127.0.0.1:3030");
        server.run().await
    }
}

#[cfg(feature = "tcp-stream")]
fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(example::run_server())
        .unwrap();
}

#[cfg(not(feature = "tcp-stream"))]
fn main() {
    println!("This example requires the 'tcp-stream' feature to be enabled.");
    println!("Run with: cargo run --example tcp_stream_server --features tcp-stream");
}
