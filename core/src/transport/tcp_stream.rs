//! TCP streaming transport implementation for JSON-RPC servers.
//!
//! Streaming TCP server for persistent connections with multiple requests per connection.

use crate::{Message, MessageProcessor};
use super::security::SecurityConfig;
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
                    && tx.send(response_json).await.is_err() {
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

    pub async fn send_message(
        &self,
        message: &Message,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(message)?;
        self.tx.send(json).await.map_err(|e| e.into())
    }

    pub async fn recv_message(
        &mut self,
    ) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        if let Some(response) = self.rx.recv().await {
            let message: Message = serde_json::from_str(&response)?;
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }
}
