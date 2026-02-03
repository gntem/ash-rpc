//! Axum HTTP transport for JSON-RPC servers.
//!
//! This module provides Axum-based HTTP transport for JSON-RPC communication.
//!
//! # Features
//! - HTTP JSON-RPC server integration with Axum
//! - Router-based setup for embedding in existing Axum applications
//! - Batch request support
//! - Error handling with proper HTTP status codes

use crate::{ErrorBuilder, Message, MessageProcessor, Response, ResponseBuilder, error_codes};
use axum::{Router, extract::State, http::StatusCode, response::Json, routing::post};
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
    match processor.process_message(message).await {
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
        if let Some(response) = processor.process_message(message).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Message, RequestBuilder, Response};
    use std::sync::Arc;

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
    fn test_axum_rpc_builder_new() {
        let builder = AxumRpcBuilder::new();
        assert!(builder.processor.is_none());
        assert_eq!(builder.path, "/rpc");
    }

    #[test]
    fn test_axum_rpc_builder_default() {
        let builder = AxumRpcBuilder::default();
        assert!(builder.processor.is_none());
        assert_eq!(builder.path, "/rpc");
    }

    #[test]
    fn test_axum_rpc_builder_processor() {
        let processor = MockProcessor;
        let builder = AxumRpcBuilder::new().processor(processor);
        assert!(builder.processor.is_some());
    }

    #[test]
    fn test_axum_rpc_builder_path() {
        let builder = AxumRpcBuilder::new().path("/custom/rpc");
        assert_eq!(builder.path, "/custom/rpc");
    }

    #[test]
    fn test_axum_rpc_builder_build_success() {
        let processor = MockProcessor;
        let builder = AxumRpcBuilder::new().processor(processor).path("/api/rpc");

        let result = builder.build();
        assert!(result.is_ok());

        let layer = result.unwrap();
        assert_eq!(layer.path, "/api/rpc");
    }

    #[test]
    fn test_axum_rpc_builder_build_no_processor() {
        let builder = AxumRpcBuilder::new();
        let result = builder.build();
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
        }
    }

    #[test]
    fn test_axum_rpc_layer_builder() {
        let _builder = AxumRpcLayer::builder();
        // Just ensure it compiles and returns a builder
    }

    #[test]
    fn test_axum_rpc_layer_into_router() {
        let processor = MockProcessor;
        let layer = AxumRpcBuilder::new()
            .processor(processor)
            .path("/rpc")
            .build()
            .unwrap();

        let _router = layer.into_router();
        // Just ensure it compiles and creates a router
    }

    #[test]
    fn test_create_rpc_router() {
        let processor = MockProcessor;
        let _router = create_rpc_router(processor, "/api");
        // Just ensure it compiles and creates a router
    }

    #[tokio::test]
    async fn test_handle_rpc_success() {
        let processor = Arc::new(MockProcessor);
        let request = RequestBuilder::new("test_method")
            .id(serde_json::Value::Number(1.into()))
            .build();
        let message = Message::Request(request);

        let result = handle_rpc(State(processor), Json(message)).await;
        assert!(result.is_ok());

        let Json(response) = result.unwrap();
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_handle_rpc_notification() {
        let processor = Arc::new(MockProcessor);
        // Create a notification (request without id)
        let notification = crate::types::Request {
            jsonrpc: "2.0".to_string(),
            method: "notify".to_string(),
            params: None,
            id: None,
            correlation_id: None,
        };
        let message = Message::Request(notification);

        let result = handle_rpc(State(processor), Json(message)).await;
        // Notifications are handled by returning a response with id: None
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_rpc_batch() {
        let processor = Arc::new(MockProcessor);
        let request1 = RequestBuilder::new("method1")
            .id(serde_json::Value::Number(1.into()))
            .build();
        let request2 = RequestBuilder::new("method2")
            .id(serde_json::Value::Number(2.into()))
            .build();

        let messages = vec![Message::Request(request1), Message::Request(request2)];

        let Json(responses) = handle_rpc_batch(State(processor), Json(messages)).await;
        assert_eq!(responses.len(), 2);
    }

    #[tokio::test]
    async fn test_handle_rpc_batch_empty() {
        let processor = Arc::new(MockProcessor);
        let messages: Vec<Message> = vec![];

        let Json(responses) = handle_rpc_batch(State(processor), Json(messages)).await;
        assert_eq!(responses.len(), 0);
    }

    #[test]
    fn test_axum_rpc_builder_chain() {
        let processor = MockProcessor;
        let builder = AxumRpcBuilder::new()
            .processor(processor)
            .path("/custom")
            .path("/override");

        let layer = builder.build().unwrap();
        assert_eq!(layer.path, "/override");
    }

    #[test]
    fn test_multiple_processors() {
        // Test that we can create multiple builders with different processors
        let processor1 = MockProcessor;
        let processor2 = MockProcessor;

        let _layer1 = AxumRpcBuilder::new().processor(processor1).build().unwrap();

        let _layer2 = AxumRpcBuilder::new()
            .processor(processor2)
            .path("/api2")
            .build()
            .unwrap();
    }

    #[tokio::test]
    async fn test_handle_rpc_batch_with_notifications() {
        let processor = Arc::new(MockProcessor);
        let request = RequestBuilder::new("method1")
            .id(serde_json::Value::Number(1.into()))
            .build();
        let notification = crate::types::Request {
            jsonrpc: "2.0".to_string(),
            method: "notify".to_string(),
            params: None,
            id: None,
            correlation_id: None,
        };

        let messages = vec![Message::Request(request), Message::Request(notification)];

        let Json(responses) = handle_rpc_batch(State(processor), Json(messages)).await;
        // Should have at least 1 response (from the request)
        assert!(!responses.is_empty());
    }
}
