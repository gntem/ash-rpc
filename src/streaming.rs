//! Streaming and subscription support for JSON-RPC.
//!
//! This module provides functionality for long-lived subscriptions and streaming responses,
//! allowing servers to push events to clients over time.

use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

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
    pub async fn subscribe(&self, request: StreamRequest) -> Result<StreamResponse, crate::Error> {
        let stream_id = request.stream_id();
        let method = request.method().to_string();

        // Get the handler for this method
        let handlers = self.handlers.read().await;
        let handler = handlers.get(&method).ok_or_else(|| {
            crate::ErrorBuilder::new(
                crate::error_codes::METHOD_NOT_FOUND,
                format!("Stream method not found: {}", method),
            )
            .build()
        })?;
        let handler = Arc::clone(handler);
        drop(handlers);

        // Call the handler to subscribe
        let response = handler
            .subscribe(request.params.clone(), stream_id.clone())
            .await?;

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
            if let Err(e) = handler
                .start_stream(stream_id_clone.clone(), request.params, event_sender)
                .await
            {
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
            crate::ErrorBuilder::new(
                crate::error_codes::INVALID_PARAMS,
                format!("Stream not found: {}", stream_id),
            )
            .build()
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
            let event = StreamEvent::new(stream_info.stream_id.clone(), method, data.clone());
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
        let id = self
            .id
            .unwrap_or_else(|| serde_json::Value::String(uuid::Uuid::new_v4().to_string()));

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_stream_request_new() {
        let id = serde_json::Value::Number(1.into());
        let request = StreamRequest::new("test_method", id.clone());

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "test_method");
        assert_eq!(request.id, id);
        assert!(request.stream_id.is_some());
        assert!(request.params.is_none());
    }

    #[test]
    fn test_stream_request_with_params() {
        let id = serde_json::Value::String("test".to_string());
        let params = json!({"key": "value"});
        let request = StreamRequest::new("method", id).with_params(params.clone());

        assert_eq!(request.params, Some(params));
    }

    #[test]
    fn test_stream_request_with_stream_id() {
        let id = serde_json::Value::Number(1.into());
        let stream_id = "custom-stream-id".to_string();
        let request = StreamRequest::new("method", id).with_stream_id(stream_id.clone());

        assert_eq!(request.stream_id, Some(stream_id));
    }

    #[test]
    fn test_stream_request_stream_id() {
        let request = StreamRequest::new("method", serde_json::Value::Null);
        let stream_id = request.stream_id();
        assert!(!stream_id.is_empty());
    }

    #[test]
    fn test_stream_request_method() {
        let request = StreamRequest::new("test_method", serde_json::Value::Null);
        assert_eq!(request.method(), "test_method");
    }

    #[test]
    fn test_stream_request_params() {
        let params = json!({"test": "data"});
        let request =
            StreamRequest::new("method", serde_json::Value::Null).with_params(params.clone());
        assert_eq!(request.params(), Some(&params));
    }

    #[test]
    fn test_stream_response_success() {
        let stream_id = "stream-123".to_string();
        let id = serde_json::Value::Number(1.into());
        let response = StreamResponse::success(stream_id.clone(), id.clone());

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert_eq!(response.id, id);
        assert_eq!(response.stream_id, stream_id);
        assert_eq!(response.stream_status, Some(StreamStatus::Active));
    }

    #[test]
    fn test_stream_response_error() {
        let error = crate::ErrorBuilder::new(100, "Test error").build();
        let stream_id = "stream-123".to_string();
        let id = serde_json::Value::Number(1.into());
        let response = StreamResponse::error(error.clone(), id.clone(), stream_id.clone());

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.id, id);
        assert_eq!(response.stream_id, stream_id);
        assert_eq!(response.stream_status, Some(StreamStatus::Error));
    }

    #[test]
    fn test_stream_response_closed() {
        let stream_id = "stream-123".to_string();
        let id = serde_json::Value::Number(1.into());
        let response = StreamResponse::closed(stream_id.clone(), id.clone());

        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert_eq!(response.stream_status, Some(StreamStatus::Closed));
    }

    #[test]
    fn test_stream_status_equality() {
        assert_eq!(StreamStatus::Active, StreamStatus::Active);
        assert_eq!(StreamStatus::Paused, StreamStatus::Paused);
        assert_eq!(StreamStatus::Closed, StreamStatus::Closed);
        assert_eq!(StreamStatus::Error, StreamStatus::Error);
        assert_ne!(StreamStatus::Active, StreamStatus::Closed);
    }

    #[test]
    fn test_stream_event_new() {
        let stream_id = "stream-123".to_string();
        let data = json!({"key": "value"});
        let event = StreamEvent::new(stream_id.clone(), "event_method", data.clone());

        assert_eq!(event.jsonrpc, "2.0");
        assert_eq!(event.method, "event_method");
        assert_eq!(event.stream_id, stream_id);
        assert_eq!(event.params, data);
        assert!(event.sequence.is_none());
    }

    #[test]
    fn test_stream_event_with_sequence() {
        let event =
            StreamEvent::new("stream-123".to_string(), "method", json!({})).with_sequence(42);
        assert_eq!(event.sequence, Some(42));
    }

    #[test]
    fn test_stream_event_stream_id() {
        let stream_id = "test-stream".to_string();
        let event = StreamEvent::new(stream_id.clone(), "method", json!({}));
        assert_eq!(event.stream_id(), stream_id);
    }

    #[test]
    fn test_stream_event_data() {
        let data = json!({"test": "data"});
        let event = StreamEvent::new("stream".to_string(), "method", data.clone());
        assert_eq!(event.data(), &data);
    }

    #[test]
    fn test_stream_event_sequence() {
        let event = StreamEvent::new("stream".to_string(), "method", json!({}));
        assert_eq!(event.sequence(), None);

        let event_with_seq = event.with_sequence(10);
        assert_eq!(event_with_seq.sequence(), Some(10));
    }

    #[test]
    fn test_unsubscribe_request_new() {
        let stream_id = "stream-123".to_string();
        let id = serde_json::Value::Number(1.into());
        let request = UnsubscribeRequest::new(stream_id.clone(), id.clone());

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "unsubscribe");
        assert_eq!(request.stream_id, stream_id);
        assert_eq!(request.id, id);
    }

    #[test]
    fn test_unsubscribe_request_stream_id() {
        let stream_id = "test-stream".to_string();
        let request = UnsubscribeRequest::new(stream_id.clone(), serde_json::Value::Null);
        assert_eq!(request.stream_id(), stream_id);
    }

    #[test]
    fn test_stream_message_is_methods() {
        let stream_req = StreamRequest::new("method", serde_json::Value::Null);
        let msg = StreamMessage::StreamRequest(stream_req);
        assert!(msg.is_stream_request());
        assert!(!msg.is_stream_response());
        assert!(!msg.is_stream_event());
        assert!(!msg.is_unsubscribe_request());

        let stream_resp = StreamResponse::success("stream".to_string(), serde_json::Value::Null);
        let msg = StreamMessage::StreamResponse(stream_resp);
        assert!(!msg.is_stream_request());
        assert!(msg.is_stream_response());

        let event = StreamEvent::new("stream".to_string(), "method", json!({}));
        let msg = StreamMessage::StreamEvent(event);
        assert!(msg.is_stream_event());

        let unsub = UnsubscribeRequest::new("stream".to_string(), serde_json::Value::Null);
        let msg = StreamMessage::UnsubscribeRequest(unsub);
        assert!(msg.is_unsubscribe_request());
    }

    #[test]
    fn test_stream_message_as_methods() {
        let stream_req = StreamRequest::new("method", serde_json::Value::Null);
        let msg = StreamMessage::StreamRequest(stream_req.clone());
        assert!(msg.as_stream_request().is_some());
        assert!(msg.as_stream_response().is_none());
        assert!(msg.as_stream_event().is_none());
    }

    #[test]
    fn test_stream_message_stream_id() {
        let stream_id = "test-stream".to_string();

        let req =
            StreamRequest::new("method", serde_json::Value::Null).with_stream_id(stream_id.clone());
        let msg = StreamMessage::StreamRequest(req);
        assert_eq!(msg.stream_id(), Some(stream_id.as_str()));
    }

    #[tokio::test]
    async fn test_stream_manager_new() {
        let manager = StreamManager::new();
        assert_eq!(manager.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_stream_manager_default() {
        let manager = StreamManager::default();
        assert_eq!(manager.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_stream_manager_active_stream_ids() {
        let manager = StreamManager::new();
        let ids = manager.active_stream_ids().await;
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn test_stream_manager_is_active() {
        let manager = StreamManager::new();
        assert!(!manager.is_active("nonexistent").await);
    }

    #[tokio::test]
    async fn test_stream_manager_get_stream_info() {
        let manager = StreamManager::new();
        let info = manager.get_stream_info("nonexistent").await;
        assert!(info.is_none());
    }

    #[tokio::test]
    async fn test_stream_manager_update_stream_status() {
        let manager = StreamManager::new();
        // This should not panic even for non-existent streams
        manager
            .update_stream_status("nonexistent", StreamStatus::Closed)
            .await;
    }

    #[tokio::test]
    async fn test_stream_manager_increment_sequence() {
        let manager = StreamManager::new();
        let result = manager.increment_sequence("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_stream_manager_broadcast_to_method() {
        let manager = StreamManager::new();
        // This should not panic
        manager
            .broadcast_to_method("test_method", json!({"data": "value"}))
            .await;
    }

    #[tokio::test]
    async fn test_stream_manager_close_all() {
        let manager = StreamManager::new();
        // This should not panic
        manager.close_all().await;
    }

    #[test]
    fn test_stream_info_creation() {
        let info = StreamInfo {
            stream_id: "stream-123".to_string(),
            method: "test_method".to_string(),
            params: Some(json!({"key": "value"})),
            created_at: std::time::Instant::now(),
            status: StreamStatus::Active,
            sequence: 0,
        };

        assert_eq!(info.stream_id, "stream-123");
        assert_eq!(info.method, "test_method");
        assert_eq!(info.status, StreamStatus::Active);
        assert_eq!(info.sequence, 0);
    }

    #[test]
    fn test_stream_request_builder_new() {
        let builder = StreamRequestBuilder::new("test_method");
        let request = builder.build();

        assert_eq!(request.method, "test_method");
        assert!(request.params.is_none());
    }

    #[test]
    fn test_stream_request_builder_with_params() {
        let params = json!({"key": "value"});
        let builder = StreamRequestBuilder::new("method").params(params.clone());
        let request = builder.build();

        assert_eq!(request.params, Some(params));
    }

    #[test]
    fn test_stream_request_builder_with_id() {
        let id = serde_json::Value::Number(42.into());
        let builder = StreamRequestBuilder::new("method").id(id.clone());
        let request = builder.build();

        assert_eq!(request.id, id);
    }

    #[test]
    fn test_stream_request_builder_with_stream_id() {
        let stream_id = "custom-stream".to_string();
        let builder = StreamRequestBuilder::new("method").stream_id(stream_id.clone());
        let request = builder.build();

        assert_eq!(request.stream_id, Some(stream_id));
    }

    #[test]
    fn test_stream_request_builder_chain() {
        let params = json!({"test": "data"});
        let id = serde_json::Value::String("test-id".to_string());
        let stream_id = "stream-123".to_string();

        let builder = StreamRequestBuilder::new("method")
            .params(params.clone())
            .id(id.clone())
            .stream_id(stream_id.clone());

        let request = builder.build();
        assert_eq!(request.method, "method");
        assert_eq!(request.params, Some(params));
        assert_eq!(request.id, id);
        assert_eq!(request.stream_id, Some(stream_id));
    }

    #[test]
    fn test_stream_status_serialization() {
        let active = serde_json::to_string(&StreamStatus::Active).unwrap();
        assert_eq!(active, "\"active\"");

        let paused = serde_json::to_string(&StreamStatus::Paused).unwrap();
        assert_eq!(paused, "\"paused\"");

        let closed = serde_json::to_string(&StreamStatus::Closed).unwrap();
        assert_eq!(closed, "\"closed\"");

        let error = serde_json::to_string(&StreamStatus::Error).unwrap();
        assert_eq!(error, "\"error\"");
    }

    #[test]
    fn test_stream_status_deserialization() {
        let active: StreamStatus = serde_json::from_str("\"active\"").unwrap();
        assert_eq!(active, StreamStatus::Active);

        let paused: StreamStatus = serde_json::from_str("\"paused\"").unwrap();
        assert_eq!(paused, StreamStatus::Paused);
    }

    #[test]
    fn test_stream_request_serialization() {
        let request = StreamRequest::new("test_method", serde_json::Value::Number(1.into()))
            .with_stream_id("stream-123".to_string());

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "test_method");
        assert_eq!(json["stream_id"], "stream-123");
    }

    #[test]
    fn test_stream_event_serialization() {
        let event = StreamEvent::new(
            "stream-123".to_string(),
            "event_method",
            json!({"key": "value"}),
        )
        .with_sequence(42);

        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "event_method");
        assert_eq!(json["stream_id"], "stream-123");
        assert_eq!(json["sequence"], 42);
    }
}
