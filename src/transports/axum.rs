//! Axum HTTP transport for JSON-RPC servers.
//!
//! This module provides Axum-based HTTP transport for JSON-RPC communication.
//!
//! # Features
//! - HTTP JSON-RPC server integration with Axum
//! - Router-based setup for embedding in existing Axum applications
//! - Batch request support
//! - Error handling with proper HTTP status codes

use crate::{
    ErrorBuilder, Message, MessageProcessor, Response, ResponseBuilder, error_codes,
};
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
