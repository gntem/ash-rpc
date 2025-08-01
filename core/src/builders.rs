//! Builder patterns for JSON-RPC types.

use crate::types::*;

/// Builder for JSON-RPC requests
pub struct RequestBuilder {
    method: String,
    params: Option<serde_json::Value>,
    id: Option<RequestId>,
}

impl RequestBuilder {
    /// Create a new request builder
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            params: None,
            id: None,
        }
    }

    /// Set request parameters
    pub fn params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Set request ID
    pub fn id(mut self, id: RequestId) -> Self {
        self.id = Some(id);
        self
    }

    /// Build the request
    pub fn build(self) -> Request {
        Request {
            jsonrpc: "2.0".to_string(),
            method: self.method,
            params: self.params,
            id: self.id,
        }
    }
}

/// Builder for JSON-RPC responses
pub struct ResponseBuilder {
    result: Option<serde_json::Value>,
    error: Option<Error>,
    id: Option<RequestId>,
}

impl ResponseBuilder {
    /// Create a new response builder
    pub fn new() -> Self {
        Self {
            result: None,
            error: None,
            id: None,
        }
    }

    /// Set successful result
    pub fn success(mut self, result: serde_json::Value) -> Self {
        self.result = Some(result);
        self
    }

    /// Set error
    pub fn error(mut self, error: Error) -> Self {
        self.error = Some(error);
        self
    }

    /// Set response ID
    pub fn id(mut self, id: Option<RequestId>) -> Self {
        self.id = id;
        self
    }

    /// Build the response
    pub fn build(self) -> Response {
        Response {
            jsonrpc: "2.0".to_string(),
            result: self.result,
            error: self.error,
            id: self.id,
        }
    }
}

/// Builder for JSON-RPC notifications
pub struct NotificationBuilder {
    method: String,
    params: Option<serde_json::Value>,
}

impl NotificationBuilder {
    /// Create a new notification builder
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            params: None,
        }
    }

    /// Set notification parameters
    pub fn params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Build the notification
    pub fn build(self) -> Notification {
        Notification {
            jsonrpc: "2.0".to_string(),
            method: self.method,
            params: self.params,
        }
    }
}

/// Builder for JSON-RPC errors
pub struct ErrorBuilder {
    code: i32,
    message: String,
    data: Option<serde_json::Value>,
}

impl ErrorBuilder {
    /// Create a new error builder
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Add additional error data
    pub fn data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Build the error
    pub fn build(self) -> Error {
        Error {
            code: self.code,
            message: self.message,
            data: self.data,
        }
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}
