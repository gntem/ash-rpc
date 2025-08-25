/// TCP transport implementation for JSON-RPC servers.
///
/// This module provides TCP-based transport layers for JSON-RPC communication,
/// including both simple TCP servers and streaming TCP servers that can handle
/// multiple requests per connection.
///
/// # Features
/// - Simple TCP server for one-request-per-connection
/// - Streaming TCP server for persistent connections
/// - Line-delimited JSON message framing
/// - Thread-based concurrent request handling
///
/// # Example
/// ```rust,no_run
/// use ash_rpc_core::{MethodRegistry, transport::tcp::TcpServerBuilder};
/// use std::sync::Arc;
///
/// let mut registry = MethodRegistry::new();
/// // ... register methods ...
/// 
/// let server = TcpServerBuilder::new("127.0.0.1:8080")
///     .processor(Arc::new(registry))
///     .build()
///     .expect("Failed to create server");
///     
/// server.run().expect("Server failed");
/// ```
#[cfg(feature = "tcp")]
pub mod tcp {
    use crate::{Message, MessageProcessor};
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;
    use std::thread;

    /// Builder for creating TCP JSON-RPC servers.
    ///
    /// Provides a fluent API for configuring and building TCP servers
    /// that can handle JSON-RPC requests over TCP connections.
    pub struct TcpServerBuilder {
        addr: String,
        processor: Option<Arc<dyn MessageProcessor + Send + Sync>>,
    }

    impl TcpServerBuilder {
        pub fn new(addr: impl Into<String>) -> Self {
            Self {
                addr: addr.into(),
                processor: None,
            }
        }

        pub fn processor<P>(mut self, processor: P) -> Self
        where
            P: MessageProcessor + Send + Sync + 'static,
        {
            self.processor = Some(Arc::new(processor));
            self
        }

        pub fn build(self) -> Result<TcpServer, std::io::Error> {
            let processor = self.processor.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Processor not set")
            })?;

            Ok(TcpServer {
                addr: self.addr,
                processor,
            })
        }
    }

    pub struct TcpServer {
        addr: String,
        processor: Arc<dyn MessageProcessor + Send + Sync>,
    }

    impl TcpServer {
        pub fn builder(addr: impl Into<String>) -> TcpServerBuilder {
            TcpServerBuilder::new(addr)
        }

        pub fn run(&self) -> Result<(), std::io::Error> {
            let listener = TcpListener::bind(&self.addr)?;
            println!("TCP RPC Server listening on {}", self.addr);

            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let processor = Arc::clone(&self.processor);
                        thread::spawn(move || {
                            if let Err(e) = handle_client(stream, processor) {
                                eprintln!("Error handling client: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Error accepting connection: {e}");
                    }
                }
            }

            Ok(())
        }
    }

    fn handle_client(
        mut stream: TcpStream,
        processor: Arc<dyn MessageProcessor + Send + Sync>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line)?;
            
            if bytes_read == 0 {
                break;
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<Message>(line) {
                Ok(message) => {
                    if let Some(response) = processor.process_message(message) {
                        let response_json = serde_json::to_string(&response)?;
                        writeln!(stream, "{response_json}")?;
                        stream.flush()?;
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
                    
                    let response_json = serde_json::to_string(&error_response)?;
                    writeln!(stream, "{response_json}")?;
                    stream.flush()?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(feature = "tcp-stream")]
pub mod tcp_stream {
    use crate::{Message, MessageProcessor};
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::mpsc;

    pub struct TcpStreamServerBuilder {
        addr: String,
        processor: Option<Arc<dyn MessageProcessor + Send + Sync>>,
    }

    impl TcpStreamServerBuilder {
        pub fn new(addr: impl Into<String>) -> Self {
            Self {
                addr: addr.into(),
                processor: None,
            }
        }

        pub fn processor<P>(mut self, processor: P) -> Self
        where
            P: MessageProcessor + Send + Sync + 'static,
        {
            self.processor = Some(Arc::new(processor));
            self
        }

        pub fn build(self) -> Result<TcpStreamServer, std::io::Error> {
            let processor = self.processor.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Processor not set")
            })?;

            Ok(TcpStreamServer {
                addr: self.addr,
                processor,
            })
        }
    }

    pub struct TcpStreamServer {
        addr: String,
        processor: Arc<dyn MessageProcessor + Send + Sync>,
    }

    impl TcpStreamServer {
        pub fn builder(addr: impl Into<String>) -> TcpStreamServerBuilder {
            TcpStreamServerBuilder::new(addr)
        }

        pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
            let listener = TcpListener::bind(&self.addr).await?;
            println!("TCP Stream RPC Server listening on {}", self.addr);

            loop {
                let (stream, addr) = listener.accept().await?;
                println!("New connection from: {addr}");
                
                let processor = Arc::clone(&self.processor);
                tokio::spawn(async move {
                    if let Err(e) = handle_stream_client(stream, processor).await {
                        eprintln!("Error handling client {addr}: {e}");
                    }
                });
            }
        }
    }

    async fn handle_stream_client(
        stream: TcpStream,
        processor: Arc<dyn MessageProcessor + Send + Sync>,
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
                    if let Some(response) = processor.process_message(message) {
                        let response_json = serde_json::to_string(&response)?;
                        if tx.send(response_json).await.is_err() {
                            break;
                        }
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
            Self {
                addr: addr.into(),
            }
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
}

#[cfg(feature = "axum")]
pub mod axum {
    use crate::{Message, MessageProcessor, Response, ResponseBuilder, ErrorBuilder, error_codes};
    use axum::{
        extract::State,
        http::StatusCode,
        response::Json,
        routing::post,
        Router,
    };
    use std::sync::Arc;

    pub struct AxumRpcBuilder {
        processor: Option<Arc<dyn MessageProcessor + Send + Sync>>,
        path: String,
    }

    impl AxumRpcBuilder {
        pub fn new() -> Self {
            Self {
                processor: None,
                path: "/rpc".to_string(),
            }
        }

        pub fn processor<P>(mut self, processor: P) -> Self
        where
            P: MessageProcessor + Send + Sync + 'static,
        {
            self.processor = Some(Arc::new(processor));
            self
        }

        pub fn path(mut self, path: impl Into<String>) -> Self {
            self.path = path.into();
            self
        }

        pub fn build(self) -> Result<AxumRpcLayer, std::io::Error> {
            let processor = self.processor.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Processor not set")
            })?;

            Ok(AxumRpcLayer {
                processor,
                path: self.path,
            })
        }
    }

    pub struct AxumRpcLayer {
        processor: Arc<dyn MessageProcessor + Send + Sync>,
        path: String,
    }

    impl AxumRpcLayer {
        pub fn builder() -> AxumRpcBuilder {
            AxumRpcBuilder::new()
        }

        pub fn into_router(self) -> Router {
            Router::new()
                .route(&self.path, post(handle_rpc))
                .with_state(self.processor)
        }
    }

    pub fn create_rpc_router<P>(processor: P, path: &str) -> Router
    where
        P: MessageProcessor + Send + Sync + 'static,
    {
        Router::new()
            .route(path, post(handle_rpc))
            .with_state(Arc::new(processor))
    }

    async fn handle_rpc(
        State(processor): State<Arc<dyn MessageProcessor + Send + Sync>>,
        Json(message): Json<Message>,
    ) -> Result<Json<Response>, (StatusCode, Json<Response>)> {
        match processor.process_message(message) {
            Some(response) => Ok(Json(response)),
            None => {
                let error_response = ResponseBuilder::new()
                    .error(
                        ErrorBuilder::new(
                            error_codes::INVALID_REQUEST,
                            "No response generated for request",
                        )
                        .build(),
                    )
                    .id(None)
                    .build();
                
                Err((StatusCode::OK, Json(error_response)))
            }
        }
    }

    pub async fn handle_rpc_batch(
        State(processor): State<Arc<dyn MessageProcessor + Send + Sync>>,
        Json(messages): Json<Vec<Message>>,
    ) -> Json<Vec<Response>> {
        let mut responses = Vec::new();
        
        for message in messages {
            if let Some(response) = processor.process_message(message) {
                responses.push(response);
            }
        }

        Json(responses)
    }

    impl Default for AxumRpcBuilder {
        fn default() -> Self {
            Self::new()
        }
    }
}
