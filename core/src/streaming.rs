//! Streaming and subscription support for JSON-RPC.
//!
//! This module provides functionality for long-lived subscriptions and streaming responses,
//! allowing servers to push events to clients over time.

use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Unique identifier for a stream/subscription
pub type StreamId = String;

/// Stream request for creating a new subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<StreamId>,
}

impl StreamRequest {
    /// Create a new stream request
    pub fn new(method: impl Into<String>, id: RequestId) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: None,
            id,
            stream_id: Some(uuid::Uuid::new_v4().to_string()),
        }
    }

    /// Add parameters to the stream request
    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Set a custom stream ID
    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    /// Get the stream ID, generating one if not present
    pub fn stream_id(&self) -> StreamId {
        self.stream_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    }

    /// Get the method name
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Get the parameters
    pub fn params(&self) -> Option<&serde_json::Value> {
        self.params.as_ref()
    }
}

/// Stream response confirming subscription creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::Error>,
    pub id: RequestId,
    pub stream_id: StreamId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_status: Option<StreamStatus>,
}

impl StreamResponse {
    /// Create a successful stream response
    pub fn success(stream_id: StreamId, id: RequestId) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "stream_id": stream_id.clone(),
                "status": "active"
            })),
            error: None,
            id,
            stream_id,
            stream_status: Some(StreamStatus::Active),
        }
    }

    /// Create an error stream response
    pub fn error(error: crate::Error, id: RequestId, stream_id: StreamId) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
            stream_id,
            stream_status: Some(StreamStatus::Error),
        }
    }

    /// Create a stream closed response
    pub fn closed(stream_id: StreamId, id: RequestId) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "stream_id": stream_id.clone(),
                "status": "closed"
            })),
            error: None,
            id,
            stream_id,
            stream_status: Some(StreamStatus::Closed),
        }
    }
}

/// Status of a stream
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StreamStatus {
    Active,
    Paused,
    Closed,
    Error,
}

/// Stream event message - data pushed from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    pub jsonrpc: String,
    pub method: String,
    pub stream_id: StreamId,
    pub params: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
}

impl StreamEvent {
    /// Create a new stream event
    pub fn new(stream_id: StreamId, method: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            stream_id,
            params: data,
            sequence: None,
        }
    }

    /// Add sequence number to the event
    pub fn with_sequence(mut self, seq: u64) -> Self {
        self.sequence = Some(seq);
        self
    }

    /// Get the stream ID
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }

    /// Get the event data
    pub fn data(&self) -> &serde_json::Value {
        &self.params
    }

    /// Get the sequence number if present
    pub fn sequence(&self) -> Option<u64> {
        self.sequence
    }
}

/// Unsubscribe request to close a stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeRequest {
    pub jsonrpc: String,
    pub method: String,
    pub stream_id: StreamId,
    pub id: RequestId,
}

impl UnsubscribeRequest {
    /// Create a new unsubscribe request
    pub fn new(stream_id: StreamId, id: RequestId) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: "unsubscribe".to_string(),
            stream_id,
            id,
        }
    }

    /// Get the stream ID
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }
}

/// Message types for streaming communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StreamMessage {
    StreamRequest(StreamRequest),
    StreamResponse(StreamResponse),
    StreamEvent(StreamEvent),
    UnsubscribeRequest(UnsubscribeRequest),
}

impl StreamMessage {
    pub fn is_stream_request(&self) -> bool {
        matches!(self, StreamMessage::StreamRequest(_))
    }

    pub fn is_stream_response(&self) -> bool {
        matches!(self, StreamMessage::StreamResponse(_))
    }

    pub fn is_stream_event(&self) -> bool {
        matches!(self, StreamMessage::StreamEvent(_))
    }

    pub fn is_unsubscribe_request(&self) -> bool {
        matches!(self, StreamMessage::UnsubscribeRequest(_))
    }

    pub fn as_stream_request(&self) -> Option<&StreamRequest> {
        match self {
            StreamMessage::StreamRequest(req) => Some(req),
            _ => None,
        }
    }

    pub fn as_stream_response(&self) -> Option<&StreamResponse> {
        match self {
            StreamMessage::StreamResponse(resp) => Some(resp),
            _ => None,
        }
    }

    pub fn as_stream_event(&self) -> Option<&StreamEvent> {
        match self {
            StreamMessage::StreamEvent(event) => Some(event),
            _ => None,
        }
    }

    pub fn stream_id(&self) -> Option<&str> {
        match self {
            StreamMessage::StreamRequest(req) => req.stream_id.as_deref(),
            StreamMessage::StreamResponse(resp) => Some(&resp.stream_id),
            StreamMessage::StreamEvent(event) => Some(&event.stream_id),
            StreamMessage::UnsubscribeRequest(req) => Some(&req.stream_id),
        }
    }
}

/// Trait for handling streaming/subscription methods
#[async_trait::async_trait]
pub trait StreamHandler: Send + Sync {
    /// Get the subscription method name this handler manages
    fn subscription_method(&self) -> &'static str;

    /// Handle a new subscription request
    async fn subscribe(
        &self,
        params: Option<serde_json::Value>,
        stream_id: StreamId,
    ) -> Result<StreamResponse, crate::Error>;

    /// Handle unsubscribe request
    async fn unsubscribe(&self, stream_id: &str) -> Result<(), crate::Error>;

    /// Start emitting events for this subscription
    /// This method should spawn a task that emits events to the provided sender
    async fn start_stream(
        &self,
        stream_id: StreamId,
        params: Option<serde_json::Value>,
        sender: mpsc::UnboundedSender<StreamEvent>,
    ) -> Result<(), crate::Error>;

    /// Check if a stream is active
    async fn is_active(&self, stream_id: &str) -> bool;
}

/// Manages multiple stream subscriptions
pub struct StreamManager {
    handlers: Arc<RwLock<HashMap<String, Arc<dyn StreamHandler>>>>,
    active_streams: Arc<RwLock<HashMap<StreamId, StreamInfo>>>,
    event_sender: mpsc::UnboundedSender<StreamEvent>,
    event_receiver: Arc<RwLock<mpsc::UnboundedReceiver<StreamEvent>>>,
}

/// Information about an active stream
#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub stream_id: StreamId,
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub created_at: std::time::Instant,
    pub status: StreamStatus,
    pub sequence: u64,
}

impl StreamManager {
    /// Create a new stream manager
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            active_streams: Arc::new(RwLock::new(HashMap::new())),
            event_sender: tx,
            event_receiver: Arc::new(RwLock::new(rx)),
        }
    }

    /// Register a stream handler
    pub async fn register_handler<H>(&self, handler: H)
    where
        H: StreamHandler + 'static,
    {
        let method = handler.subscription_method().to_string();
        let handler_arc = Arc::new(handler);
        
        let mut handlers = self.handlers.write().await;
        handlers.insert(method.clone(), handler_arc);
        
        tracing::debug!(method = %method, "stream handler registered");
    }

    /// Subscribe to a stream
    pub async fn subscribe(
        &self,
        request: StreamRequest,
    ) -> Result<StreamResponse, crate::Error> {
        let stream_id = request.stream_id();
        let method = request.method().to_string();
        
        // Get the handler for this method
        let handlers = self.handlers.read().await;
        let handler = handlers.get(&method).ok_or_else(|| {
            crate::Error::new(
                crate::error_codes::METHOD_NOT_FOUND,
                format!("Stream method not found: {}", method),
            )
        })?;
        let handler = Arc::clone(handler);
        drop(handlers);

        // Call the handler to subscribe
        let response = handler.subscribe(request.params.clone(), stream_id.clone()).await?;

        // Store stream info
        let stream_info = StreamInfo {
            stream_id: stream_id.clone(),
            method: method.clone(),
            params: request.params.clone(),
            created_at: std::time::Instant::now(),
            status: StreamStatus::Active,
            sequence: 0,
        };
        
        let mut streams = self.active_streams.write().await;
        streams.insert(stream_id.clone(), stream_info);
        drop(streams);

        // Start the stream in the background
        let event_sender = self.event_sender.clone();
        let stream_id_clone = stream_id.clone();
        tokio::spawn(async move {
            if let Err(e) = handler.start_stream(stream_id_clone.clone(), request.params, event_sender).await {
                tracing::error!(stream_id = %stream_id_clone, error = ?e, "stream failed");
            }
        });

        tracing::info!(stream_id = %stream_id, method = %method, "stream subscribed");
        Ok(response)
    }

    /// Unsubscribe from a stream
    pub async fn unsubscribe(&self, stream_id: &str) -> Result<(), crate::Error> {
        // Get stream info
        let streams = self.active_streams.read().await;
        let stream_info = streams.get(stream_id).ok_or_else(|| {
            crate::Error::new(
                crate::error_codes::INVALID_PARAMS,
                format!("Stream not found: {}", stream_id),
            )
        })?;
        
        let method = stream_info.method.clone();
        drop(streams);

        // Get handler and unsubscribe
        let handlers = self.handlers.read().await;
        if let Some(handler) = handlers.get(&method) {
            handler.unsubscribe(stream_id).await?;
        }
        drop(handlers);

        // Remove from active streams
        let mut streams = self.active_streams.write().await;
        streams.remove(stream_id);
        drop(streams);

        tracing::info!(stream_id = %stream_id, method = %method, "stream unsubscribed");
        Ok(())
    }

    /// Get next event from any active stream
    pub async fn next_event(&self) -> Option<StreamEvent> {
        let mut receiver = self.event_receiver.write().await;
        receiver.recv().await
    }

    /// Get all active stream IDs
    pub async fn active_stream_ids(&self) -> Vec<StreamId> {
        let streams = self.active_streams.read().await;
        streams.keys().cloned().collect()
    }

    /// Get stream info
    pub async fn get_stream_info(&self, stream_id: &str) -> Option<StreamInfo> {
        let streams = self.active_streams.read().await;
        streams.get(stream_id).cloned()
    }

    /// Check if a stream is active
    pub async fn is_active(&self, stream_id: &str) -> bool {
        let streams = self.active_streams.read().await;
        streams.contains_key(stream_id)
    }

    /// Get count of active streams
    pub async fn active_count(&self) -> usize {
        let streams = self.active_streams.read().await;
        streams.len()
    }

    /// Close all streams
    pub async fn close_all(&self) {
        let stream_ids: Vec<_> = {
            let streams = self.active_streams.read().await;
            streams.keys().cloned().collect()
        };

        for stream_id in stream_ids {
            let _ = self.unsubscribe(&stream_id).await;
        }
        
        tracing::info!("all streams closed");
    }

    /// Update stream status
    pub async fn update_stream_status(&self, stream_id: &str, status: StreamStatus) {
        let mut streams = self.active_streams.write().await;
        if let Some(stream_info) = streams.get_mut(stream_id) {
            stream_info.status = status;
        }
    }

    /// Increment stream sequence
    pub async fn increment_sequence(&self, stream_id: &str) -> Option<u64> {
        let mut streams = self.active_streams.write().await;
        if let Some(stream_info) = streams.get_mut(stream_id) {
            stream_info.sequence += 1;
            Some(stream_info.sequence)
        } else {
            None
        }
    }

    /// Broadcast event to all subscribers of a method
    pub async fn broadcast_to_method(&self, method: &str, data: serde_json::Value) {
        let streams = self.active_streams.read().await;
        let matching_streams: Vec<_> = streams
            .values()
            .filter(|info| info.method == method && info.status == StreamStatus::Active)
            .collect();

        for stream_info in matching_streams {
            let sequence = self.increment_sequence(&stream_info.stream_id).await;
            let event = StreamEvent::new(
                stream_info.stream_id.clone(),
                method,
                data.clone(),
            );
            let event = if let Some(seq) = sequence {
                event.with_sequence(seq)
            } else {
                event
            };

            if self.event_sender.send(event).is_err() {
                tracing::error!(stream_id = %stream_info.stream_id, "failed to send event");
            }
        }
    }
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating stream requests
pub struct StreamRequestBuilder {
    method: String,
    params: Option<serde_json::Value>,
    id: Option<RequestId>,
    stream_id: Option<StreamId>,
}

impl StreamRequestBuilder {
    /// Create a new stream request builder
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            params: None,
            id: None,
            stream_id: None,
        }
    }

    /// Set the parameters
    pub fn params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Set the request ID
    pub fn id(mut self, id: RequestId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set a custom stream ID
    pub fn stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    /// Build the stream request
    pub fn build(self) -> StreamRequest {
        let id = self.id.unwrap_or_else(|| {
            serde_json::Value::String(uuid::Uuid::new_v4().to_string())
        });
        
        let mut request = StreamRequest::new(self.method, id);
        
        if let Some(params) = self.params {
            request = request.with_params(params);
        }
        
        if let Some(stream_id) = self.stream_id {
            request = request.with_stream_id(stream_id);
        }
        
        request
    }
}

/// Helper macro for creating stream events
#[macro_export]
macro_rules! stream_event {
    ($stream_id:expr, $method:expr, $data:expr) => {
        $crate::StreamEvent::new($stream_id, $method, serde_json::json!($data))
    };
}
