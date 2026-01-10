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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

impl Request {
    /// Create a new JSON-RPC request
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: None,
            id: None,
            correlation_id: Some(uuid::Uuid::new_v4().to_string()),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

impl Response {
    /// Create a successful response
    pub fn success(result: serde_json::Value, id: Option<RequestId>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
            correlation_id: None,
        }
    }

    /// Create an error response
    pub fn error(error: crate::Error, id: Option<RequestId>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
            correlation_id: None,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    /// Transform this error using a custom callback function
    ///
    /// This allows library users to implement their own error sanitization
    /// logic based on their security requirements.
    ///
    /// # Example
    /// ```ignore
    /// let sanitized = error.sanitized_with(|err| {
    ///     if err.code() == INTERNAL_ERROR {
    ///         Error::new(err.code(), "Internal server error")
    ///     } else {
    ///         err.clone()
    ///     }
    /// });
    /// ```
    pub fn sanitized_with<F>(&self, transform: F) -> Self
    where
        F: FnOnce(&Self) -> Self,
    {
        transform(self)
    }

    /// Create a generic internal error from any std::error::Error
    ///
    /// This logs the full error details server-side and returns a generic error.
    /// Use this with sanitized_with() for custom error transformation.
    pub fn from_error_logged(error: &dyn std::error::Error) -> Self {
        tracing::error!(
            error = %error,
            error_debug = ?error,
            "internal error occurred"
        );

        Self {
            code: crate::error_codes::INTERNAL_ERROR,
            message: "Internal server error".to_string(),
            data: None,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Request tests
    #[test]
    fn test_request_creation() {
        let request = Request::new("test_method");
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "test_method");
        assert!(request.params.is_none());
        assert!(request.id.is_none());
        assert!(request.correlation_id.is_some());
    }

    #[test]
    fn test_request_with_params() {
        let params = json!({"key": "value"});
        let request = Request::new("method").with_params(params.clone());
        assert_eq!(request.params(), Some(&params));
    }

    #[test]
    fn test_request_with_id() {
        let id = json!(42);
        let request = Request::new("method").with_id(id.clone());
        assert_eq!(request.id(), Some(&id));
        assert!(request.expects_response());
        assert!(!request.is_notification());
    }

    #[test]
    fn test_request_notification() {
        let request = Request::new("notify");
        assert!(!request.expects_response());
        assert!(request.is_notification());
    }

    #[test]
    fn test_request_serialization() {
        let request = Request::new("test")
            .with_params(json!([1, 2, 3]))
            .with_id(json!(1));

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&serialized).unwrap();

        assert_eq!(request.method, deserialized.method);
        assert_eq!(request.params, deserialized.params);
        assert_eq!(request.id, deserialized.id);
    }

    #[test]
    fn test_request_take_params() {
        let params = json!([1, 2, 3]);
        let request = Request::new("test").with_params(params.clone());
        let taken = request.take_params();
        assert_eq!(taken, Some(params));
    }

    // Response tests
    #[test]
    fn test_response_success() {
        let result = json!({"status": "ok"});
        let response = Response::success(result.clone(), Some(json!(1)));

        assert!(response.is_success());
        assert!(!response.is_error());
        assert_eq!(response.result(), Some(&result));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_response_error() {
        let error = Error::new(-32600, "Invalid request");
        let response = Response::error(error.clone(), Some(json!(1)));

        assert!(!response.is_success());
        assert!(response.is_error());
        assert!(response.result.is_none());
        assert_eq!(response.error_info().unwrap().code, error.code);
    }

    #[test]
    fn test_response_serialization() {
        let response = Response::success(json!("result"), Some(json!(1)));
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();

        assert_eq!(response.result, deserialized.result);
        assert_eq!(response.id, deserialized.id);
    }

    #[test]
    fn test_response_take_result() {
        let result = json!({"data": "value"});
        let response = Response::success(result.clone(), Some(json!(1)));
        let taken = response.take_result();
        assert_eq!(taken, Some(result));
    }

    #[test]
    fn test_response_take_error() {
        let error = Error::new(-32600, "Error");
        let response = Response::error(error.clone(), Some(json!(1)));
        let taken = response.take_error();
        assert!(taken.is_some());
        assert_eq!(taken.unwrap().code(), error.code());
    }

    // Error tests
    #[test]
    fn test_error_creation() {
        let error = Error::new(-32600, "Test error");
        assert_eq!(error.code(), -32600);
        assert_eq!(error.message(), "Test error");
        assert!(error.data().is_none());
    }

    #[test]
    fn test_error_with_data() {
        let data = json!({"details": "more info"});
        let error = Error::new(-32000, "Error").with_data(data.clone());
        assert_eq!(error.data(), Some(&data));
    }

    #[test]
    fn test_error_type_checks() {
        assert!(Error::new(error_codes::PARSE_ERROR, "msg").is_parse_error());
        assert!(Error::new(error_codes::INVALID_REQUEST, "msg").is_invalid_request());
        assert!(Error::new(error_codes::METHOD_NOT_FOUND, "msg").is_method_not_found());
        assert!(Error::new(error_codes::INVALID_PARAMS, "msg").is_invalid_params());
        assert!(Error::new(error_codes::INTERNAL_ERROR, "msg").is_internal_error());
        assert!(Error::new(-32001, "msg").is_server_error());
        assert!(!Error::new(-32700, "msg").is_server_error());
    }

    #[test]
    fn test_error_sanitization() {
        let error = Error::new(
            -32603,
            "Internal database connection failed: postgres://user:pass@host",
        );
        let sanitized = error.sanitized_with(|e| Error::new(e.code, "Internal server error"));

        assert_eq!(sanitized.code(), error.code());
        assert_eq!(sanitized.message(), "Internal server error");
        assert!(!sanitized.message().contains("postgres"));
    }

    #[test]
    fn test_error_from_std_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error = Error::from_error_logged(&io_error);

        assert_eq!(error.code(), error_codes::INTERNAL_ERROR);
        assert_eq!(error.message(), "Internal server error");
    }

    // Notification tests
    #[test]
    fn test_notification_creation() {
        let notification = Notification::new("notify");
        assert_eq!(notification.jsonrpc, "2.0");
        assert_eq!(notification.method(), "notify");
        assert!(notification.params().is_none());
    }

    #[test]
    fn test_notification_with_params() {
        let params = json!({"event": "update"});
        let notification = Notification::new("notify").with_params(params.clone());
        assert_eq!(notification.params(), Some(&params));
    }

    #[test]
    fn test_notification_serialization() {
        let notification = Notification::new("event").with_params(json!([1, 2]));
        let serialized = serde_json::to_string(&notification).unwrap();
        let deserialized: Notification = serde_json::from_str(&serialized).unwrap();

        assert_eq!(notification.method, deserialized.method);
        assert_eq!(notification.params, deserialized.params);
    }

    #[test]
    fn test_notification_take_params() {
        let params = json!({"event": "data"});
        let notification = Notification::new("notify").with_params(params.clone());
        let taken = notification.take_params();
        assert_eq!(taken, Some(params));
    }

    // Message tests
    #[test]
    fn test_message_request_variant() {
        let request = Request::new("test");
        let message = Message::Request(request);

        assert!(message.is_request());
        assert!(!message.is_response());
        assert!(!message.is_notification());
    }

    #[test]
    fn test_message_response_variant() {
        let response = Response::success(json!("ok"), Some(json!(1)));
        let message = Message::Response(response);

        assert!(!message.is_request());
        assert!(message.is_response());
        assert!(!message.is_notification());
    }

    #[test]
    fn test_message_notification_variant() {
        let notification = Notification::new("event");
        let message = Message::Notification(notification);

        assert!(!message.is_request());
        assert!(!message.is_response());
        assert!(message.is_notification());
    }

    #[test]
    fn test_message_serialization_request() {
        let request = Request::new("test").with_id(json!(1));
        let message = Message::Request(request);

        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();

        assert!(deserialized.is_request());
    }

    // Additional Message method tests
    #[test]
    fn test_message_as_request() {
        let request = Request::new("test");
        let message = Message::Request(request.clone());

        assert!(message.as_request().is_some());
        assert_eq!(message.as_request().unwrap().method, "test");
        assert!(message.as_response().is_none());
        assert!(message.as_notification().is_none());
    }

    #[test]
    fn test_message_as_response() {
        let response = Response::success(json!(42), Some(json!(1)));
        let message = Message::Response(response);

        assert!(message.as_response().is_some());
        assert!(message.as_request().is_none());
        assert!(message.as_notification().is_none());
    }

    #[test]
    fn test_message_as_notification() {
        let notification = Notification::new("event");
        let message = Message::Notification(notification);

        assert!(message.as_notification().is_some());
        assert_eq!(message.as_notification().unwrap().method, "event");
        assert!(message.as_request().is_none());
        assert!(message.as_response().is_none());
    }

    #[test]
    fn test_message_into_request() {
        let request = Request::new("test");
        let message = Message::Request(request);

        let extracted = message.into_request();
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap().method, "test");
    }

    #[test]
    fn test_message_into_response() {
        let response = Response::success(json!(true), Some(json!(1)));
        let message = Message::Response(response);

        let extracted = message.into_response();
        assert!(extracted.is_some());
        assert!(extracted.unwrap().is_success());
    }

    #[test]
    fn test_message_into_notification() {
        let notification = Notification::new("notify");
        let message = Message::Notification(notification);

        let extracted = message.into_notification();
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap().method, "notify");
    }

    #[test]
    fn test_message_into_wrong_type() {
        let message = Message::Request(Request::new("test"));
        assert!(message.clone().into_response().is_none());
        assert!(message.into_notification().is_none());
    }

    #[test]
    fn test_message_method_from_request() {
        let request = Request::new("my_method");
        let message = Message::Request(request);
        assert_eq!(message.method(), Some("my_method"));
    }

    #[test]
    fn test_message_method_from_notification() {
        let notification = Notification::new("event_method");
        let message = Message::Notification(notification);
        assert_eq!(message.method(), Some("event_method"));
    }

    #[test]
    fn test_message_method_from_response() {
        let response = Response::success(json!(1), Some(json!(1)));
        let message = Message::Response(response);
        assert_eq!(message.method(), None);
    }

    #[test]
    fn test_message_id_from_request() {
        let request = Request::new("test").with_id(json!(123));
        let message = Message::Request(request);
        assert_eq!(message.id(), Some(&json!(123)));
    }

    #[test]
    fn test_message_id_from_response() {
        let response = Response::success(json!(1), Some(json!("abc")));
        let message = Message::Response(response);
        assert_eq!(message.id(), Some(&json!("abc")));
    }

    #[test]
    fn test_message_id_from_notification() {
        let notification = Notification::new("event");
        let message = Message::Notification(notification);
        assert_eq!(message.id(), None);
    }

    #[test]
    fn test_message_id_none() {
        let request = Request::new("test"); // No ID
        let message = Message::Request(request);
        assert_eq!(message.id(), None);
    }

    // Additional Request tests
    #[test]
    fn test_request_method_accessor() {
        let request = Request::new("get_data");
        assert_eq!(request.method(), "get_data");
    }

    #[test]
    fn test_request_params_accessor() {
        let params = json!({"key": "value"});
        let request = Request::new("test").with_params(params.clone());
        assert_eq!(request.params(), Some(&params));
    }

    #[test]
    fn test_request_params_none() {
        let request = Request::new("test");
        assert_eq!(request.params(), None);
    }

    #[test]
    fn test_request_id_accessor() {
        let request = Request::new("test").with_id(json!(999));
        assert_eq!(request.id(), Some(&json!(999)));
    }

    // Additional Response tests
    #[test]
    fn test_response_result_accessor() {
        let result = json!({"data": "value"});
        let response = Response::success(result.clone(), Some(json!(1)));
        assert_eq!(response.result(), Some(&result));
    }

    #[test]
    fn test_response_error_info() {
        let error = Error::new(-32600, "Invalid Request");
        let response = Response::error(error.clone(), Some(json!(1)));
        assert!(response.error_info().is_some());
        assert_eq!(response.error_info().unwrap().code, -32600);
    }

    #[test]
    fn test_response_id_accessor() {
        let response = Response::success(json!(1), Some(json!("req-id")));
        assert_eq!(response.id(), Some(&json!("req-id")));
    }

    // Additional Error tests
    #[test]
    fn test_error_code_accessor() {
        let error = Error::new(-32001, "Custom error");
        assert_eq!(error.code(), -32001);
    }

    #[test]
    fn test_error_message_accessor() {
        let error = Error::new(-32002, "Test message");
        assert_eq!(error.message(), "Test message");
    }

    #[test]
    fn test_error_data_accessor() {
        let data = json!({"detail": "info"});
        let error = Error::new(-32003, "Error").with_data(data.clone());
        assert_eq!(error.data(), Some(&data));
    }

    #[test]
    fn test_error_data_none() {
        let error = Error::new(-32004, "Error");
        assert_eq!(error.data(), None);
    }

    #[test]
    fn test_error_is_invalid_params() {
        let error = Error::new(error_codes::INVALID_PARAMS, "Invalid");
        assert!(error.is_invalid_params());
        assert!(!error.is_parse_error());
    }

    #[test]
    fn test_error_is_internal_error() {
        let error = Error::new(error_codes::INTERNAL_ERROR, "Internal");
        assert!(error.is_internal_error());
        assert!(!error.is_server_error());
    }

    #[test]
    fn test_error_is_server_error() {
        let error = Error::new(-32050, "Server error");
        assert!(error.is_server_error());
        assert!(!error.is_internal_error());

        // Boundary tests
        let error_min = Error::new(-32099, "Min");
        assert!(error_min.is_server_error());

        let error_max = Error::new(-32000, "Max");
        assert!(error_max.is_server_error());

        let error_out = Error::new(-31999, "Out of range");
        assert!(!error_out.is_server_error());
    }

    #[test]
    fn test_error_sanitized_with() {
        let error = Error::new(-32603, "Database connection failed: host=db.internal");
        let sanitized = error.sanitized_with(|e| Error::new(e.code(), "Internal server error"));

        assert_eq!(sanitized.code(), -32603);
        assert_eq!(sanitized.message(), "Internal server error");
        assert!(!sanitized.message().contains("db.internal"));
    }

    // Additional Notification tests
    #[test]
    fn test_notification_method_accessor() {
        let notification = Notification::new("user_logged_in");
        assert_eq!(notification.method(), "user_logged_in");
    }

    #[test]
    fn test_notification_params_accessor() {
        let params = json!({"user_id": 123});
        let notification = Notification::new("event").with_params(params.clone());
        assert_eq!(notification.params(), Some(&params));
    }

    #[test]
    fn test_notification_params_none() {
        let notification = Notification::new("ping");
        assert_eq!(notification.params(), None);
    }

    // Error constants tests
    #[test]
    fn test_error_code_constants() {
        assert_eq!(error_codes::PARSE_ERROR, -32700);
        assert_eq!(error_codes::INVALID_REQUEST, -32600);
        assert_eq!(error_codes::METHOD_NOT_FOUND, -32601);
        assert_eq!(error_codes::INVALID_PARAMS, -32602);
        assert_eq!(error_codes::INTERNAL_ERROR, -32603);
    }

    #[test]
    fn test_error_from_std_error_logging() {
        use std::io;
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let error = Error::from_error_logged(&io_error);

        assert_eq!(error.code(), error_codes::INTERNAL_ERROR);
        assert_eq!(error.message(), "Internal server error");
    }

    #[test]
    fn test_request_correlation_id() {
        let request = Request::new("test");
        // Auto-generated correlation ID
        assert!(request.correlation_id.is_some());
    }

    #[test]
    fn test_response_with_correlation_id() {
        let mut response = Response::success(json!(1), Some(json!(1)));
        response.correlation_id = Some("custom-id".to_string());
        assert_eq!(response.correlation_id, Some("custom-id".to_string()));
    }
}
