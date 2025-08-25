#[cfg(feature = "tcp-stream")]
mod example {
    use ash_rpc_core::{transport::tcp_stream::TcpStreamClientBuilder, Message, RequestBuilder};

    pub async fn run_client() -> Result<(), Box<dyn std::error::Error>> {
        println!("Connecting to TCP Stream server...");
        let mut client = TcpStreamClientBuilder::new("127.0.0.1:3030")
            .connect()
            .await?;

        println!("Connected! Sending requests...");

        let request = RequestBuilder::new("echo")
            .params(serde_json::json!({
                "message": "Hello from TCP Stream client!"
            }))
            .id(serde_json::json!(1))
            .build();

        client.send_message(&Message::Request(request)).await?;

        if let Some(response) = client.recv_message().await? {
            println!(
                "Received response: {}",
                serde_json::to_string_pretty(&response)?
            );
        }

        let request2 = RequestBuilder::new("multiply")
            .params(serde_json::json!({
                "a": 5,
                "b": 10
            }))
            .id(serde_json::json!(2))
            .build();

        client.send_message(&Message::Request(request2)).await?;

        if let Some(response) = client.recv_message().await? {
            println!(
                "Received response: {}",
                serde_json::to_string_pretty(&response)?
            );
        }

        Ok(())
    }
}

#[cfg(feature = "tcp-stream")]
fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(example::run_client())
        .unwrap();
}

#[cfg(not(feature = "tcp-stream"))]
fn main() {
    println!("This example requires the 'tcp-stream' feature to be enabled.");
    println!("Run with: cargo run --example tcp_stream_client --features tcp-stream");
}
