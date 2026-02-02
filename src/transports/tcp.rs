//! TCP transport implementation for JSON-RPC servers.
//!
//! Simple TCP server for one-request-per-connection pattern.

use super::security::SecurityConfig;
use crate::{Message, MessageProcessor};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;
use tokio::time::timeout;

/// Builder for creating TCP JSON-RPC servers.
///
/// Provides a fluent API for configuring and building TCP servers
/// that can handle JSON-RPC requests over TCP connections.
pub struct TcpServerBuilder {
    addr: String,
    processor: Option<Arc<dyn MessageProcessor + Send + Sync>>,
    security_config: SecurityConfig,
}

impl TcpServerBuilder {
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

    pub fn build(self) -> Result<TcpServer, std::io::Error> {
        let processor = self.processor.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Processor not set")
        })?;

        Ok(TcpServer {
            addr: self.addr,
            processor,
            security_config: self.security_config,
            active_connections: Arc::new(AtomicUsize::new(0)),
        })
    }
}

pub struct TcpServer {
    addr: String,
    processor: Arc<dyn MessageProcessor + Send + Sync>,
    security_config: SecurityConfig,
    active_connections: Arc<AtomicUsize>,
}

impl TcpServer {
    pub fn builder(addr: impl Into<String>) -> TcpServerBuilder {
        TcpServerBuilder::new(addr)
    }

    pub fn run(&self) -> Result<(), std::io::Error> {
        let rt = Runtime::new()?;
        rt.block_on(self.run_async())
    }

    async fn run_async(&self) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(&self.addr).await?;
        tracing::info!(
            addr = %self.addr,
            protocol = "tcp",
            max_connections = self.security_config.max_connections,
            max_request_size = self.security_config.max_request_size,
            "server listening"
        );

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
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
                    let processor = Arc::clone(&self.processor);
                    let security_config = self.security_config.clone();
                    let active_connections = Arc::clone(&self.active_connections);

                    tokio::spawn(async move {
                        let result = handle_client(stream, processor, security_config).await;
                        active_connections.fetch_sub(1, Ordering::Relaxed);

                        if let Err(e) = result {
                            tracing::error!(remote_addr = %addr, error = %e, "client handler failed");
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(error = %e, "failed to accept connection");
                }
            }
        }
    }
}

async fn handle_client(
    stream: TcpStream,
    processor: Arc<dyn MessageProcessor + Send + Sync>,
    security_config: SecurityConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();

        // Apply request timeout
        let bytes_read =
            match timeout(security_config.request_timeout, reader.read_line(&mut line)).await {
                Ok(result) => result?,
                Err(_) => {
                    tracing::warn!("request timeout exceeded");
                    return Err("request timeout".into());
                }
            };

        // Check max request size
        if security_config.max_request_size > 0 && line.len() > security_config.max_request_size {
            tracing::warn!(
                request_size = line.len(),
                max_size = security_config.max_request_size,
                "request size limit exceeded"
            );
            let error_response = crate::Response::error(
                crate::ErrorBuilder::new(
                    crate::error_codes::INVALID_REQUEST,
                    "Request size limit exceeded".to_string(),
                )
                .build(),
                None,
            );
            if let Ok(json) = serde_json::to_string(&error_response) {
                let _ = writer.write_all(json.as_bytes()).await;
                let _ = writer.write_all(b"\n").await;
            }
            return Err("request size limit exceeded".into());
        }

        if bytes_read == 0 {
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match serde_json::from_str::<Message>(line) {
            Ok(message) => {
                let response_opt = processor.process_message(message).await;
                if let Some(response) = response_opt {
                    let response_json = serde_json::to_string(&response)?;
                    writer.write_all(response_json.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                }
            }
            Err(e) => {
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

                let error_json = serde_json::to_string(&error_response)?;
                writer.write_all(error_json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, Request, Response, error_codes};
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    // Mock processor for testing
    struct MockProcessor;

    #[async_trait::async_trait]
    impl MessageProcessor for MockProcessor {
        async fn process_message(&self, message: Message) -> Option<Response> {
            match message {
                Message::Request(req) => {
                    if req.method == "echo" {
                        Some(Response::success(
                            req.params.unwrap_or(serde_json::json!(null)),
                            req.id,
                        ))
                    } else if req.method == "error" {
                        Some(Response::error(
                            crate::ErrorBuilder::new(
                                crate::error_codes::INTERNAL_ERROR,
                                "Test error",
                            )
                            .build(),
                            req.id,
                        ))
                    } else {
                        Some(Response::error(
                            crate::ErrorBuilder::new(
                                error_codes::METHOD_NOT_FOUND,
                                "Method not found",
                            )
                            .build(),
                            req.id,
                        ))
                    }
                }
                Message::Notification(_) => None,
                Message::Response(resp) => Some(resp),
            }
        }
    }

    // Builder tests
    #[test]
    fn test_tcp_server_builder_new() {
        let builder = TcpServerBuilder::new("127.0.0.1:8080");
        assert_eq!(builder.addr, "127.0.0.1:8080");
        assert!(builder.processor.is_none());
    }

    #[test]
    fn test_tcp_server_builder_with_processor() {
        let builder = TcpServerBuilder::new("127.0.0.1:8080").processor(MockProcessor);
        assert!(builder.processor.is_some());
    }

    #[test]
    fn test_tcp_server_builder_with_security_config() {
        let config = SecurityConfig {
            max_connections: 50,
            max_request_size: 2048,
            request_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(60),
        };
        let builder = TcpServerBuilder::new("127.0.0.1:8080").security_config(config.clone());
        assert_eq!(builder.security_config.max_connections, 50);
        assert_eq!(builder.security_config.max_request_size, 2048);
    }

    #[test]
    fn test_tcp_server_builder_max_connections() {
        let builder = TcpServerBuilder::new("127.0.0.1:8080").max_connections(100);
        assert_eq!(builder.security_config.max_connections, 100);
    }

    #[test]
    fn test_tcp_server_builder_max_request_size() {
        let builder = TcpServerBuilder::new("127.0.0.1:8080").max_request_size(4096);
        assert_eq!(builder.security_config.max_request_size, 4096);
    }

    #[test]
    fn test_tcp_server_builder_request_timeout() {
        let timeout_val = Duration::from_secs(20);
        let builder = TcpServerBuilder::new("127.0.0.1:8080").request_timeout(timeout_val);
        assert_eq!(builder.security_config.request_timeout, timeout_val);
    }

    #[test]
    fn test_tcp_server_builder_build_without_processor() {
        let builder = TcpServerBuilder::new("127.0.0.1:8080");
        let result = builder.build();
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn test_tcp_server_builder_build_success() {
        let builder = TcpServerBuilder::new("127.0.0.1:8080").processor(MockProcessor);
        let result = builder.build();
        assert!(result.is_ok());
        let server = result.unwrap();
        assert_eq!(server.addr, "127.0.0.1:8080");
    }

    #[test]
    fn test_tcp_server_builder_chaining() {
        let builder = TcpServerBuilder::new("127.0.0.1:8080")
            .processor(MockProcessor)
            .max_connections(200)
            .max_request_size(8192)
            .request_timeout(Duration::from_secs(30));

        assert_eq!(builder.security_config.max_connections, 200);
        assert_eq!(builder.security_config.max_request_size, 8192);
        assert_eq!(
            builder.security_config.request_timeout,
            Duration::from_secs(30)
        );
    }

    #[test]
    fn test_tcp_server_builder_method() {
        let builder = TcpServer::builder("127.0.0.1:9000");
        assert_eq!(builder.addr, "127.0.0.1:9000");
    }

    #[test]
    fn test_tcp_server_active_connections_initial() {
        let server = TcpServer::builder("127.0.0.1:8080")
            .processor(MockProcessor)
            .build()
            .unwrap();
        assert_eq!(server.active_connections.load(Ordering::Relaxed), 0);
    }

    // Integration tests with actual TCP connections
    #[tokio::test]
    async fn test_tcp_server_echo_request() {
        let server = TcpServer::builder("127.0.0.1:0")
            .processor(MockProcessor)
            .build()
            .unwrap();

        let listener = TcpListener::bind(&server.addr).await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn server
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let processor = Arc::new(MockProcessor);
            let config = SecurityConfig::default();
            let _ = handle_client(stream, processor, config).await;
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Connect and send request
        let mut client = TcpStream::connect(addr).await.unwrap();
        let request = Request::new("echo").with_params(serde_json::json!({"msg": "hello"}));
        let request_json = serde_json::to_string(&Message::Request(request)).unwrap();
        client.write_all(request_json.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
        client.flush().await.unwrap();

        // Read response
        let mut response = String::new();
        let mut reader = BufReader::new(client);
        reader.read_line(&mut response).await.unwrap();

        let resp: Response = serde_json::from_str(&response).unwrap();
        assert!(resp.result.is_some());
        assert_eq!(resp.result.unwrap(), serde_json::json!({"msg": "hello"}));
    }

    #[tokio::test]
    async fn test_tcp_server_error_response() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let processor = Arc::new(MockProcessor);
            let config = SecurityConfig::default();
            let _ = handle_client(stream, processor, config).await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        let request = Request::new("error");
        let request_json = serde_json::to_string(&Message::Request(request)).unwrap();
        client.write_all(request_json.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
        client.flush().await.unwrap();

        let mut response = String::new();
        let mut reader = BufReader::new(client);
        reader.read_line(&mut response).await.unwrap();

        let resp: Response = serde_json::from_str(&response).unwrap();
        assert!(resp.error.is_some());
        let error = resp.error.unwrap();
        assert_eq!(error.code, crate::error_codes::INTERNAL_ERROR);
        assert_eq!(error.message, "Test error");
    }

    #[tokio::test]
    async fn test_tcp_server_parse_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let processor = Arc::new(MockProcessor);
            let config = SecurityConfig::default();
            let _ = handle_client(stream, processor, config).await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        client.write_all(b"invalid json\n").await.unwrap();
        client.flush().await.unwrap();

        let mut response = String::new();
        let mut reader = BufReader::new(client);
        reader.read_line(&mut response).await.unwrap();

        let resp: Response = serde_json::from_str(&response).unwrap();
        assert!(resp.error.is_some());
        let error = resp.error.unwrap();
        assert_eq!(error.code, error_codes::PARSE_ERROR);
        assert!(error.message.contains("Parse error"));
    }

    #[tokio::test]
    async fn test_tcp_server_notification() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let processor = Arc::new(MockProcessor);
            let config = SecurityConfig::default();
            let _ = handle_client(stream, processor, config).await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        let notification = crate::Notification::new("notify");
        let notif_json = serde_json::to_string(&Message::Notification(notification)).unwrap();
        client.write_all(notif_json.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
        client.flush().await.unwrap();

        // Notification should not produce a response
        client.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_tcp_server_empty_lines() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let processor = Arc::new(MockProcessor);
            let config = SecurityConfig::default();
            let _ = handle_client(stream, processor, config).await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        // Send empty lines
        client.write_all(b"\n\n\n").await.unwrap();

        // Then send a valid request
        let request = Request::new("echo").with_params(serde_json::json!(42));
        let request_json = serde_json::to_string(&Message::Request(request)).unwrap();
        client.write_all(request_json.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
        client.flush().await.unwrap();

        let mut response = String::new();
        let mut reader = BufReader::new(client);
        reader.read_line(&mut response).await.unwrap();

        let resp: Response = serde_json::from_str(&response).unwrap();
        assert_eq!(resp.result.unwrap(), serde_json::json!(42));
    }

    #[tokio::test]
    async fn test_tcp_server_request_size_limit() {
        let config = SecurityConfig {
            max_connections: 100,
            max_request_size: 50, // Very small limit
            request_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(60),
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let processor = Arc::new(MockProcessor);
            let _ = handle_client(stream, processor, config).await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        // Send a request larger than 50 bytes
        let request = Request::new("echo")
            .with_params(serde_json::json!({"very": "long", "data": "that exceeds the limit"}));
        let request_json = serde_json::to_string(&Message::Request(request)).unwrap();
        client.write_all(request_json.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
        client.flush().await.unwrap();

        let mut response = String::new();
        let mut reader = BufReader::new(client);
        reader.read_line(&mut response).await.unwrap();

        let resp: Response = serde_json::from_str(&response).unwrap();
        assert!(resp.error.is_some());
        let error = resp.error.unwrap();
        assert_eq!(error.code, error_codes::INVALID_REQUEST);
        assert!(error.message.contains("size limit exceeded"));
    }

    #[tokio::test]
    async fn test_tcp_server_request_timeout() {
        let config = SecurityConfig {
            max_connections: 100,
            max_request_size: 1024 * 1024,
            request_timeout: Duration::from_millis(100), // Very short timeout
            idle_timeout: Duration::from_secs(60),
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let processor = Arc::new(MockProcessor);
            let _ = handle_client(stream, processor, config).await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        // Don't send anything, just wait for timeout
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Connection should be closed by server due to timeout
        let mut buf = [0u8; 1024];
        let result = client.read(&mut buf).await;
        // Connection should be closed (read returns 0 or error)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tcp_server_method_not_found() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let processor = Arc::new(MockProcessor);
            let config = SecurityConfig::default();
            let _ = handle_client(stream, processor, config).await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        let request = Request::new("nonexistent_method");
        let request_json = serde_json::to_string(&Message::Request(request)).unwrap();
        client.write_all(request_json.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
        client.flush().await.unwrap();

        let mut response = String::new();
        let mut reader = BufReader::new(client);
        reader.read_line(&mut response).await.unwrap();

        let resp: Response = serde_json::from_str(&response).unwrap();
        assert!(resp.error.is_some());
        let error = resp.error.unwrap();
        assert_eq!(error.code, error_codes::METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_tcp_server_multiple_requests() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let processor = Arc::new(MockProcessor);
            let config = SecurityConfig::default();
            let _ = handle_client(stream, processor, config).await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        let (read_half, mut write_half) = client.split();
        let mut reader = BufReader::new(read_half);

        // Send first request
        let request1 = Request::new("echo").with_params(serde_json::json!(1));
        let request_json1 = serde_json::to_string(&Message::Request(request1)).unwrap();
        write_half
            .write_all(request_json1.as_bytes())
            .await
            .unwrap();
        write_half.write_all(b"\n").await.unwrap();
        write_half.flush().await.unwrap();

        let mut response1 = String::new();
        reader.read_line(&mut response1).await.unwrap();
        let resp1: Response = serde_json::from_str(&response1).unwrap();
        assert_eq!(resp1.result.unwrap(), serde_json::json!(1));

        // Send second request
        let request2 = Request::new("echo").with_params(serde_json::json!(2));
        let request_json2 = serde_json::to_string(&Message::Request(request2)).unwrap();
        write_half
            .write_all(request_json2.as_bytes())
            .await
            .unwrap();
        write_half.write_all(b"\n").await.unwrap();
        write_half.flush().await.unwrap();

        let mut response2 = String::new();
        reader.read_line(&mut response2).await.unwrap();
        let resp2: Response = serde_json::from_str(&response2).unwrap();
        assert_eq!(resp2.result.unwrap(), serde_json::json!(2));
    }

    #[tokio::test]
    async fn test_tcp_server_addr_string_conversion() {
        let addr_str = String::from("127.0.0.1:7777");
        let builder = TcpServerBuilder::new(addr_str.clone());
        assert_eq!(builder.addr, addr_str);
    }

    #[tokio::test]
    async fn test_tcp_server_zero_max_request_size() {
        let config = SecurityConfig {
            max_connections: 100,
            max_request_size: 0, // Zero means no limit
            request_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(60),
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let processor = Arc::new(MockProcessor);
            let _ = handle_client(stream, processor, config).await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        let request = Request::new("echo").with_params(serde_json::json!({"data": "some data"}));
        let request_json = serde_json::to_string(&Message::Request(request)).unwrap();
        client.write_all(request_json.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
        client.flush().await.unwrap();

        let mut response = String::new();
        let mut reader = BufReader::new(client);
        reader.read_line(&mut response).await.unwrap();

        let resp: Response = serde_json::from_str(&response).unwrap();
        assert!(resp.result.is_some());
    }
}
