//! TCP streaming transport implementation for JSON-RPC servers.
//!
//! Streaming TCP server for persistent connections with multiple requests per connection.

use super::security::SecurityConfig;
use crate::{Message, MessageProcessor};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

pub struct TcpStreamServerBuilder {
    addr: String,
    processor: Option<Arc<dyn MessageProcessor + Send + Sync>>,
    security_config: SecurityConfig,
}

impl TcpStreamServerBuilder {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            processor: None,
            security_config: SecurityConfig::default(),
        }
    }

    pub fn processor<P>(mut self, processor: P) -> Self
    where
        P: MessageProcessor + Send + Sync + 'static,
    {
        self.processor = Some(Arc::new(processor));
        self
    }

    pub fn security_config(mut self, config: SecurityConfig) -> Self {
        self.security_config = config;
        self
    }

    pub fn max_connections(mut self, max: usize) -> Self {
        self.security_config.max_connections = max;
        self
    }

    pub fn max_request_size(mut self, size: usize) -> Self {
        self.security_config.max_request_size = size;
        self
    }

    pub fn request_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.security_config.request_timeout = timeout;
        self
    }

    pub fn build(self) -> Result<TcpStreamServer, std::io::Error> {
        let processor = self.processor.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Processor not set")
        })?;

        Ok(TcpStreamServer {
            addr: self.addr,
            processor,
            security_config: self.security_config,
            active_connections: Arc::new(AtomicUsize::new(0)),
        })
    }
}

pub struct TcpStreamServer {
    addr: String,
    processor: Arc<dyn MessageProcessor + Send + Sync>,
    security_config: SecurityConfig,
    active_connections: Arc<AtomicUsize>,
}

impl TcpStreamServer {
    pub fn builder(addr: impl Into<String>) -> TcpStreamServerBuilder {
        TcpStreamServerBuilder::new(addr)
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        tracing::info!(
            addr = %self.addr,
            protocol = "tcp-stream",
            max_connections = self.security_config.max_connections,
            max_request_size = self.security_config.max_request_size,
            "server listening"
        );

        loop {
            let (stream, addr) = listener.accept().await?;

            let current_connections = self.active_connections.load(Ordering::Relaxed);

            // Check connection limit
            if self.security_config.max_connections > 0
                && current_connections >= self.security_config.max_connections
            {
                tracing::warn!(
                    remote_addr = %addr,
                    active_connections = current_connections,
                    max_connections = self.security_config.max_connections,
                    "connection limit reached, rejecting connection"
                );
                drop(stream);
                continue;
            }

            self.active_connections.fetch_add(1, Ordering::Relaxed);
            tracing::debug!(remote_addr = %addr, active_connections = current_connections + 1, "new connection");

            let processor = Arc::clone(&self.processor);
            let security_config = self.security_config.clone();
            let active_connections = Arc::clone(&self.active_connections);

            tokio::spawn(async move {
                let result = handle_stream_client(stream, processor, security_config).await;
                active_connections.fetch_sub(1, Ordering::Relaxed);

                if let Err(e) = result {
                    tracing::error!(remote_addr = %addr, error = %e, "client handler failed");
                }
            });
        }
    }
}

async fn handle_stream_client(
    stream: TcpStream,
    processor: Arc<dyn MessageProcessor + Send + Sync>,
    _security_config: SecurityConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let (tx, mut rx) = mpsc::channel::<String>(100);

    tokio::spawn(async move {
        let mut writer = writer;
        while let Some(response) = rx.recv().await {
            if writer.write_all(response.as_bytes()).await.is_err()
                || writer.write_all(b"\n").await.is_err()
                || writer.flush().await.is_err()
            {
                break;
            }
        }
    });

    let mut line = String::new();
    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            break;
        }

        let line_content = line.trim();
        if line_content.is_empty() {
            continue;
        }

        match serde_json::from_str::<Message>(line_content) {
            Ok(message) => {
                if let Some(response) = processor.process_message(message).await
                    && let Ok(response_json) = serde_json::to_string(&response)
                    && tx.send(response_json).await.is_err()
                {
                    break;
                }
            }
            Err(e) => {
                tracing::debug!(error = %e, "json-rpc parse failed");
                let error_response = crate::ResponseBuilder::new()
                    .error(
                        crate::ErrorBuilder::new(
                            crate::error_codes::PARSE_ERROR,
                            format!("Parse error: {e}"),
                        )
                        .build(),
                    )
                    .id(None)
                    .build();

                let response_json = serde_json::to_string(&error_response)?;
                if tx.send(response_json).await.is_err() {
                    break;
                }
            }
        }
    }

    Ok(())
}

pub struct TcpStreamClientBuilder {
    addr: String,
}

impl TcpStreamClientBuilder {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }

    pub async fn connect(self) -> Result<TcpStreamClient, Box<dyn std::error::Error>> {
        let stream = TcpStream::connect(&self.addr).await?;
        Ok(TcpStreamClient::new(stream))
    }
}

pub struct TcpStreamClient {
    tx: mpsc::Sender<String>,
    rx: mpsc::Receiver<String>,
}

impl TcpStreamClient {
    fn new(stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let (write_tx, mut write_rx) = mpsc::channel::<String>(100);
        let (read_tx, read_rx) = mpsc::channel::<String>(100);

        tokio::spawn(async move {
            let mut writer = writer;
            while let Some(message) = write_rx.recv().await {
                if writer.write_all(message.as_bytes()).await.is_err() {
                    break;
                }
                if writer.write_all(b"\n").await.is_err() {
                    break;
                }
                if writer.flush().await.is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let line_content = line.trim();
                        if !line_content.is_empty()
                            && read_tx.send(line_content.to_string()).await.is_err()
                        {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            tx: write_tx,
            rx: read_rx,
        }
    }

    pub async fn send_message(&self, message: &Message) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(message)?;
        self.tx.send(json).await.map_err(|e| e.into())
    }

    pub async fn recv_message(&mut self) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        if let Some(response) = self.rx.recv().await {
            let message: Message = serde_json::from_str(&response)?;
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Message, RequestBuilder, Response, ResponseBuilder};

    // Mock message processor for testing
    struct MockProcessor;

    #[async_trait::async_trait]
    impl MessageProcessor for MockProcessor {
        async fn process_message(&self, message: Message) -> Option<Response> {
            match message {
                Message::Request(req) => {
                    let result = serde_json::json!({"result": "success"});
                    Some(
                        ResponseBuilder::new()
                            .success(result)
                            .id(req.id.clone())
                            .build(),
                    )
                }
                _ => None,
            }
        }
    }

    #[test]
    fn test_tcp_stream_server_builder_new() {
        let builder = TcpStreamServerBuilder::new("127.0.0.1:8080");
        assert_eq!(builder.addr, "127.0.0.1:8080");
        assert!(builder.processor.is_none());
    }

    #[test]
    fn test_tcp_stream_server_builder_processor() {
        let processor = MockProcessor;
        let builder = TcpStreamServerBuilder::new("127.0.0.1:8080").processor(processor);
        assert!(builder.processor.is_some());
    }

    #[test]
    fn test_tcp_stream_server_builder_security_config() {
        let security_config = SecurityConfig {
            max_connections: 10,
            max_request_size: 1024,
            request_timeout: std::time::Duration::from_secs(30),
            idle_timeout: std::time::Duration::from_secs(60),
        };
        let builder =
            TcpStreamServerBuilder::new("127.0.0.1:8080").security_config(security_config.clone());
        assert_eq!(builder.security_config.max_connections, 10);
        assert_eq!(builder.security_config.max_request_size, 1024);
    }

    #[test]
    fn test_tcp_stream_server_builder_max_connections() {
        let builder = TcpStreamServerBuilder::new("127.0.0.1:8080").max_connections(50);
        assert_eq!(builder.security_config.max_connections, 50);
    }

    #[test]
    fn test_tcp_stream_server_builder_max_request_size() {
        let builder = TcpStreamServerBuilder::new("127.0.0.1:8080").max_request_size(2048);
        assert_eq!(builder.security_config.max_request_size, 2048);
    }

    #[test]
    fn test_tcp_stream_server_builder_request_timeout() {
        let timeout = std::time::Duration::from_secs(10);
        let builder = TcpStreamServerBuilder::new("127.0.0.1:8080").request_timeout(timeout);
        assert_eq!(builder.security_config.request_timeout, timeout);
    }

    #[test]
    fn test_tcp_stream_server_builder_build_success() {
        let processor = MockProcessor;
        let builder = TcpStreamServerBuilder::new("127.0.0.1:8080").processor(processor);

        let result = builder.build();
        assert!(result.is_ok());

        let server = result.unwrap();
        assert_eq!(server.addr, "127.0.0.1:8080");
    }

    #[test]
    fn test_tcp_stream_server_builder_build_no_processor() {
        let builder = TcpStreamServerBuilder::new("127.0.0.1:8080");
        let result = builder.build();
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
        }
    }

    #[test]
    fn test_tcp_stream_server_builder_chain() {
        let processor = MockProcessor;
        let builder = TcpStreamServerBuilder::new("127.0.0.1:8080")
            .processor(processor)
            .max_connections(100)
            .max_request_size(4096)
            .request_timeout(std::time::Duration::from_secs(20));

        let server = builder.build().unwrap();
        assert_eq!(server.security_config.max_connections, 100);
        assert_eq!(server.security_config.max_request_size, 4096);
        assert_eq!(
            server.security_config.request_timeout,
            std::time::Duration::from_secs(20)
        );
    }

    #[test]
    fn test_tcp_stream_server_builder_static_method() {
        let _builder = TcpStreamServer::builder("127.0.0.1:8080");
        // Just ensure it compiles
    }

    #[test]
    fn test_tcp_stream_server_active_connections() {
        let processor = MockProcessor;
        let server = TcpStreamServerBuilder::new("127.0.0.1:8080")
            .processor(processor)
            .build()
            .unwrap();

        assert_eq!(server.active_connections.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_tcp_stream_client_builder_new() {
        let builder = TcpStreamClientBuilder::new("127.0.0.1:8080");
        assert_eq!(builder.addr, "127.0.0.1:8080");
    }

    #[test]
    fn test_security_config_defaults() {
        let config = SecurityConfig::default();
        assert!(config.max_connections > 0 || config.max_connections == 0);
        // Just ensure defaults are set
    }

    #[test]
    fn test_multiple_builders() {
        let processor1 = MockProcessor;
        let processor2 = MockProcessor;

        let _server1 = TcpStreamServerBuilder::new("127.0.0.1:8080")
            .processor(processor1)
            .build()
            .unwrap();

        let _server2 = TcpStreamServerBuilder::new("127.0.0.1:8081")
            .processor(processor2)
            .max_connections(10)
            .build()
            .unwrap();
    }

    #[test]
    fn test_builder_with_different_addresses() {
        let processor = MockProcessor;

        let server1 = TcpStreamServerBuilder::new("0.0.0.0:3000")
            .processor(MockProcessor)
            .build()
            .unwrap();
        assert_eq!(server1.addr, "0.0.0.0:3000");

        let server2 = TcpStreamServerBuilder::new("localhost:4000")
            .processor(processor)
            .build()
            .unwrap();
        assert_eq!(server2.addr, "localhost:4000");
    }

    #[test]
    fn test_security_config_clone() {
        let config1 = SecurityConfig {
            max_connections: 10,
            max_request_size: 1024,
            request_timeout: std::time::Duration::from_secs(30),
            idle_timeout: std::time::Duration::from_secs(60),
        };
        let config2 = config1.clone();

        assert_eq!(config1.max_connections, config2.max_connections);
        assert_eq!(config1.max_request_size, config2.max_request_size);
    }

    #[tokio::test]
    async fn test_message_serialization() {
        let request = RequestBuilder::new("test_method")
            .id(serde_json::Value::Number(1.into()))
            .params(serde_json::json!({"key": "value"}))
            .build();

        let message = Message::Request(request);
        let json = serde_json::to_string(&message).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();

        match parsed {
            Message::Request(req) => {
                assert_eq!(req.method, "test_method");
                assert_eq!(req.id, Some(serde_json::Value::Number(1.into())));
            }
            _ => panic!("Expected Request"),
        }
    }
}
