//! Core JSON-RPC 2.0 types and data structures.

use serde::{Deserialize, Serialize};

/// Request identifier - can be string, number, or null
pub type RequestId = serde_json::Value;

/// JSON-RPC 2.0 request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
}

impl Request {
    /// Create a new JSON-RPC request
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: None,
            id: None,
        }
    }

    /// Add parameters to the request
    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Add an ID to the request
    pub fn with_id(mut self, id: RequestId) -> Self {
        self.id = Some(id);
        self
    }

    /// Check if this request expects a response
    pub fn expects_response(&self) -> bool {
        self.id.is_some()
    }

    /// Check if this is a notification (no response expected)
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }

    /// Get the method name
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Get a reference to the parameters
    pub fn params(&self) -> Option<&serde_json::Value> {
        self.params.as_ref()
    }

    /// Take ownership of the parameters
    pub fn take_params(self) -> Option<serde_json::Value> {
        self.params
    }

    /// Get a reference to the request ID
    pub fn id(&self) -> Option<&RequestId> {
        self.id.as_ref()
    }
}

/// JSON-RPC 2.0 response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::Error>,
    pub id: Option<RequestId>,
}

impl Response {
    /// Create a successful response
    pub fn success(result: serde_json::Value, id: Option<RequestId>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response
    pub fn error(error: crate::Error, id: Option<RequestId>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }

    /// Check if this is a successful response
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }

    /// Check if this is an error response
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Get a reference to the result
    pub fn result(&self) -> Option<&serde_json::Value> {
        self.result.as_ref()
    }

    /// Take ownership of the result
    pub fn take_result(self) -> Option<serde_json::Value> {
        self.result
    }

    /// Get error information
    pub fn error_info(&self) -> Option<&crate::Error> {
        self.error.as_ref()
    }

    /// Take ownership of the error
    pub fn take_error(self) -> Option<crate::Error> {
        self.error
    }

    /// Get the response ID
    pub fn id(&self) -> Option<&RequestId> {
        self.id.as_ref()
    }
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Error {
    /// Create a new error
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Add additional data to the error
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Check if this is a parse error (-32700)
    pub fn is_parse_error(&self) -> bool {
        self.code == crate::error_codes::PARSE_ERROR
    }

    /// Check if this is an invalid request error (-32600)
    pub fn is_invalid_request(&self) -> bool {
        self.code == crate::error_codes::INVALID_REQUEST
    }

    /// Check if this is a method not found error (-32601)
    pub fn is_method_not_found(&self) -> bool {
        self.code == crate::error_codes::METHOD_NOT_FOUND
    }

    pub fn is_invalid_params(&self) -> bool {
        self.code == crate::error_codes::INVALID_PARAMS
    }

    pub fn is_internal_error(&self) -> bool {
        self.code == crate::error_codes::INTERNAL_ERROR
    }

    pub fn is_server_error(&self) -> bool {
        self.code >= -32099 && self.code <= -32000
    }

    pub fn code(&self) -> i32 {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn data(&self) -> Option<&serde_json::Value> {
        self.data.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl Notification {
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: None,
        }
    }

    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn params(&self) -> Option<&serde_json::Value> {
        self.params.as_ref()
    }

    pub fn take_params(self) -> Option<serde_json::Value> {
        self.params
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

impl Message {
    pub fn is_request(&self) -> bool {
        matches!(self, Message::Request(_))
    }

    pub fn is_response(&self) -> bool {
        matches!(self, Message::Response(_))
    }

    pub fn is_notification(&self) -> bool {
        matches!(self, Message::Notification(_))
    }

    pub fn as_request(&self) -> Option<&Request> {
        match self {
            Message::Request(req) => Some(req),
            _ => None,
        }
    }

    pub fn as_response(&self) -> Option<&Response> {
        match self {
            Message::Response(resp) => Some(resp),
            _ => None,
        }
    }

    pub fn as_notification(&self) -> Option<&Notification> {
        match self {
            Message::Notification(notif) => Some(notif),
            _ => None,
        }
    }

    pub fn into_request(self) -> Option<Request> {
        match self {
            Message::Request(req) => Some(req),
            _ => None,
        }
    }

    pub fn into_response(self) -> Option<Response> {
        match self {
            Message::Response(resp) => Some(resp),
            _ => None,
        }
    }

    pub fn into_notification(self) -> Option<Notification> {
        match self {
            Message::Notification(notif) => Some(notif),
            _ => None,
        }
    }

    pub fn method(&self) -> Option<&str> {
        match self {
            Message::Request(req) => Some(&req.method),
            Message::Notification(notif) => Some(&notif.method),
            Message::Response(_) => None,
        }
    }

    pub fn id(&self) -> Option<&RequestId> {
        match self {
            Message::Request(req) => req.id.as_ref(),
            Message::Response(resp) => resp.id.as_ref(),
            Message::Notification(_) => None,
        }
    }
}

/// Standard JSON-RPC 2.0 error codes as defined in the specification.
///
/// These constants provide the standard error codes that should be used
/// for common JSON-RPC error conditions. Using these ensures compliance
/// with the JSON-RPC 2.0 specification.
///
/// # Example
/// ```rust
/// use ash_rpc_core::{ErrorBuilder, error_codes};
///
/// let error = ErrorBuilder::new(error_codes::METHOD_NOT_FOUND, "Method not found")
///     .build();
/// ```
pub mod error_codes {
    /// Parse error - Invalid JSON was received by the server.
    /// An error occurred on the server while parsing the JSON text.
    pub const PARSE_ERROR: i32 = -32700;

    /// Invalid Request - The JSON sent is not a valid Request object.
    pub const INVALID_REQUEST: i32 = -32600;

    /// Method not found - The method does not exist / is not available.
    pub const METHOD_NOT_FOUND: i32 = -32601;

    /// Invalid params - Invalid method parameter(s).
    pub const INVALID_PARAMS: i32 = -32602;

    /// Internal error - Internal JSON-RPC error.
    pub const INTERNAL_ERROR: i32 = -32603;
}
