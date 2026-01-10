//! WebSocket transport for JSON-RPC servers and clients.
//!
//! This module provides WebSocket-based transport for JSON-RPC communication.
//!
//! # Features
//! - WebSocket JSON-RPC server for persistent connections
//! - WebSocket JSON-RPC client for connecting to servers
//! - Support for both text and binary WebSocket messages
//! - Automatic ping/pong handling
//! - Concurrent connection handling

use ash_rpc_core::{
    ErrorBuilder, Message, MessageProcessor, Response, ResponseBuilder, error_codes,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

/// Builder for creating WebSocket JSON-RPC servers.
///
/// Provides a fluent API for configuring and building WebSocket servers
/// that can handle JSON-RPC requests over WebSocket connections.
pub struct WebSocketServerBuilder {
    addr: String,
    processor: Option<Arc<dyn MessageProcessor + Send + Sync>>,
}

impl WebSocketServerBuilder {
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

    pub fn build(self) -> Result<WebSocketServer, std::io::Error> {
        let processor = self.processor.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Processor not set")
        })?;

        Ok(WebSocketServer {
            addr: self.addr,
            processor,
        })
    }
}

/// WebSocket JSON-RPC server.
///
/// Accepts WebSocket connections and processes JSON-RPC messages.
/// Supports both single requests and persistent connections with multiple requests.
pub struct WebSocketServer {
    addr: String,
    processor: Arc<dyn MessageProcessor + Send + Sync>,
}

impl WebSocketServer {
    pub fn builder(addr: impl Into<String>) -> WebSocketServerBuilder {
        WebSocketServerBuilder::new(addr)
    }

    /// Run the WebSocket server.
    ///
    /// This method blocks and listens for incoming WebSocket connections,
    /// spawning a new task for each connection.
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        println!("WebSocket RPC Server listening on {}", self.addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            println!("New WebSocket connection from: {addr}");

            let processor = Arc::clone(&self.processor);
            tokio::spawn(async move {
                if let Err(e) = handle_websocket_connection(stream, processor).await {
                    eprintln!("Error handling WebSocket client {addr}: {e}");
                }
            });
        }
    }
}

async fn handle_websocket_connection(
    stream: TcpStream,
    processor: Arc<dyn MessageProcessor + Send + Sync>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws_stream = accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();

    let (tx, mut rx) = mpsc::channel::<String>(100);

    // Spawn task to send responses
    tokio::spawn(async move {
        while let Some(response) = rx.recv().await {
            if write.send(WsMessage::Text(response.into())).await.is_err() {
                break;
            }
        }
    });

    // Process incoming messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(WsMessage::Text(text)) => match serde_json::from_str::<Message>(&text) {
                Ok(message) => {
                    if let Some(response) = processor.process_message(message).await {
                        let response_json = serde_json::to_string(&response)?;
                        if tx.send(response_json).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let error_response = ResponseBuilder::new()
                        .error(
                            ErrorBuilder::new(
                                error_codes::PARSE_ERROR,
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
            },
            Ok(WsMessage::Binary(data)) => match serde_json::from_slice::<Message>(&data) {
                Ok(message) => {
                    if let Some(response) = processor.process_message(message).await {
                        let response_json = serde_json::to_string(&response)?;
                        if tx.send(response_json).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let error_response = ResponseBuilder::new()
                        .error(
                            ErrorBuilder::new(
                                error_codes::PARSE_ERROR,
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
            },
            Ok(WsMessage::Close(_)) => {
                break;
            }
            Ok(WsMessage::Ping(data)) => {
                // Respond to ping with pong
                if tx
                    .send(format!("{{\"pong\":{}}}", data.len()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Ok(WsMessage::Pong(_)) => {
                // Ignore pong messages
            }
            Err(e) => {
                eprintln!("WebSocket error: {e}");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Builder for creating WebSocket JSON-RPC clients.
pub struct WebSocketClientBuilder {
    url: String,
}

impl WebSocketClientBuilder {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }

    pub async fn connect(self) -> Result<WebSocketClient, Box<dyn std::error::Error>> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(&self.url).await?;
        Ok(WebSocketClient::new(ws_stream))
    }
}

/// WebSocket JSON-RPC client.
///
/// Connects to a WebSocket server and can send JSON-RPC requests
/// and receive responses.
pub struct WebSocketClient {
    tx: mpsc::Sender<String>,
    rx: mpsc::Receiver<String>,
}

impl WebSocketClient {
    fn new(
        ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>,
    ) -> Self {
        let (mut write, mut read) = ws_stream.split();
        let (write_tx, mut write_rx) = mpsc::channel::<String>(100);
        let (read_tx, read_rx) = mpsc::channel::<String>(100);

        // Spawn task to send messages
        tokio::spawn(async move {
            while let Some(message) = write_rx.recv().await {
                if write.send(WsMessage::Text(message.into())).await.is_err() {
                    break;
                }
            }
        });

        // Spawn task to receive messages
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => {
                        if read_tx.send(text.to_string()).await.is_err() {
                            break;
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        break;
                    }
                    Err(_) => {
                        break;
                    }
                    _ => {}
                }
            }
        });

        Self {
            tx: write_tx,
            rx: read_rx,
        }
    }

    /// Send a JSON-RPC message to the server.
    pub async fn send_message(&self, message: &Message) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(message)?;
        self.tx.send(json).await.map_err(|e| e.into())
    }

    /// Receive a response from the server.
    pub async fn recv_response(&mut self) -> Result<Option<Response>, Box<dyn std::error::Error>> {
        if let Some(response) = self.rx.recv().await {
            let parsed: Response = serde_json::from_str(&response)?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Receive any message from the server.
    pub async fn recv_message(&mut self) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        if let Some(response) = self.rx.recv().await {
            let message: Message = serde_json::from_str(&response)?;
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }
}
