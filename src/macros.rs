//! Convenience macros for JSON-RPC response creation.

/// Create a success response with a result value and optional ID
///
/// # Examples:
/// ```text
/// // Success with ID
/// rpc_success!(42, Some(1))
///
/// // Success with ID from variable
/// rpc_success!(result, id)
///
/// // Success without ID
/// rpc_success!({"status": "ok"})
/// ```
#[macro_export]
macro_rules! rpc_success {
    ($result:expr_2021, $id:expr_2021) => {
        $crate::ResponseBuilder::new()
            .success(serde_json::json!($result))
            .id($id)
            .build()
    };
    ($result:expr_2021) => {
        $crate::ResponseBuilder::new()
            .success(serde_json::json!($result))
            .id(None)
            .build()
    };
}

/// Create an error response with code, message, and optional ID
///
/// # Usage:
/// ```text
/// // Error with explicit code, message and ID
/// rpc_error!(error_codes::INVALID_PARAMS, "Invalid parameters", Some(1))
///
/// // Error with code and message, ID from variable
/// rpc_error!(error_codes::INVALID_PARAMS, "Invalid parameters", id)
///
/// // Error without ID
/// rpc_error!(error_codes::METHOD_NOT_FOUND, "Method not found")
///
/// // Error using predefined error codes
/// rpc_error!(error_codes::INVALID_PARAMS, "Invalid parameters", id)
/// ```
#[macro_export]
macro_rules! rpc_error {
    ($code:expr_2021, $message:expr_2021, $id:expr_2021) => {
        $crate::ResponseBuilder::new()
            .error($crate::ErrorBuilder::new($code, $message).build())
            .id($id)
            .build()
    };
    ($code:expr_2021, $message:expr_2021) => {
        $crate::ResponseBuilder::new()
            .error($crate::ErrorBuilder::new($code, $message).build())
            .id(None)
            .build()
    };
}

/// Create an error response with code, message, additional data and optional ID
///
/// # Usage:
/// ```text
/// // Error with data
/// rpc_error_with_data!(error_codes::INVALID_PARAMS, "Invalid parameters", {"expected": "array"}, Some(1))
///
/// // Error with data but no ID
/// rpc_error_with_data!(error_codes::INVALID_PARAMS, "Invalid parameters", {"expected": "array"})
/// ```
#[macro_export]
macro_rules! rpc_error_with_data {
    ($code:expr_2021, $message:expr_2021, $data:expr_2021, $id:expr_2021) => {
        $crate::ResponseBuilder::new()
            .error(
                $crate::ErrorBuilder::new($code, $message)
                    .data(serde_json::json!($data))
                    .build(),
            )
            .id($id)
            .build()
    };
    ($code:expr_2021, $message:expr_2021, $data:expr_2021) => {
        $crate::ResponseBuilder::new()
            .error(
                $crate::ErrorBuilder::new($code, $message)
                    .data(serde_json::json!($data))
                    .build(),
            )
            .id(None)
            .build()
    };
}

/// Common error response shortcuts using predefined error codes
///
/// # Usage:
/// ```text
/// rpc_invalid_params!("Expected array of two numbers", id)
/// rpc_method_not_found!(id)
/// rpc_parse_error!("Invalid JSON", id)
/// rpc_internal_error!("Database connection failed", id)
/// ```
#[macro_export]
macro_rules! rpc_invalid_params {
    ($message:expr_2021, $id:expr_2021) => {
        rpc_error!($crate::error_codes::INVALID_PARAMS, $message, $id)
    };
    ($message:expr_2021) => {
        rpc_error!($crate::error_codes::INVALID_PARAMS, $message)
    };
}

#[macro_export]
macro_rules! rpc_method_not_found {
    ($id:expr_2021) => {
        rpc_error!(
            $crate::error_codes::METHOD_NOT_FOUND,
            "Method not found",
            $id
        )
    };
    () => {
        rpc_error!($crate::error_codes::METHOD_NOT_FOUND, "Method not found")
    };
}

#[macro_export]
macro_rules! rpc_parse_error {
    ($message:expr_2021, $id:expr_2021) => {
        rpc_error!($crate::error_codes::PARSE_ERROR, $message, $id)
    };
    ($message:expr_2021) => {
        rpc_error!($crate::error_codes::PARSE_ERROR, $message)
    };
}

#[macro_export]
macro_rules! rpc_internal_error {
    ($message:expr_2021, $id:expr_2021) => {
        rpc_error!($crate::error_codes::INTERNAL_ERROR, $message, $id)
    };
    ($message:expr_2021) => {
        rpc_error!($crate::error_codes::INTERNAL_ERROR, $message)
    };
}

/// Create a JSON-RPC request
///
/// # Usage:
/// ```text
/// // Request with method, params and ID
/// rpc_request!("add", [5, 3], 1)
///
/// // Request with method and ID (no params)
/// rpc_request!("ping", 2)
///
/// // Request with method only (notification - no ID)
/// rpc_request!("log")
/// ```
#[macro_export]
macro_rules! rpc_request {
    ($method:expr_2021, $params:expr_2021, $id:expr_2021) => {
        $crate::RequestBuilder::new($method)
            .params(serde_json::json!($params))
            .id(serde_json::json!($id))
            .build()
    };
    ($method:expr_2021, $id:expr_2021) => {
        $crate::RequestBuilder::new($method)
            .id(serde_json::json!($id))
            .build()
    };
    ($method:expr_2021) => {
        $crate::RequestBuilder::new($method).build()
    };
}

/// Create a JSON-RPC notification
///
/// # Usage:
/// ```text
/// // Notification with method and params
/// rpc_notification!("log", {"level": "info", "message": "Hello"})
///
/// // Notification with method only
/// rpc_notification!("ping")
/// ```
#[macro_export]
macro_rules! rpc_notification {
    ($method:expr_2021, $params:expr_2021) => {
        $crate::NotificationBuilder::new($method)
            .params(serde_json::json!($params))
            .build()
    };
    ($method:expr_2021) => {
        $crate::NotificationBuilder::new($method).build()
    };
}

/// Create a JSON-RPC error object (not a response)
///
/// # Usage:
/// ```text
/// // Error with code and message
/// rpc_error_obj!(error_codes::INVALID_PARAMS, "Invalid parameters")
///
/// // Error with code, message and data
/// rpc_error_obj!(error_codes::INVALID_PARAMS, "Invalid parameters", {"expected": "array"})
/// ```
#[macro_export]
macro_rules! rpc_error_obj {
    ($code:expr_2021, $message:expr_2021, $data:expr_2021) => {
        $crate::ErrorBuilder::new($code, $message)
            .data(serde_json::json!($data))
            .build()
    };
    ($code:expr_2021, $message:expr_2021) => {
        $crate::ErrorBuilder::new($code, $message).build()
    };
}

//
// Transport Macros - Easy server creation
//

/// Create and run a TCP JSON-RPC server
///
/// # Usage:
/// ```text
/// // Basic TCP server with registry
/// rpc_tcp_server!("127.0.0.1:8080", registry);
///
/// // TCP server with error handling
/// rpc_tcp_server!("127.0.0.1:8080", registry).expect("Failed to start server");
/// ```
#[cfg(feature = "tcp")]
#[macro_export]
macro_rules! rpc_tcp_server {
    ($addr:expr_2021, $processor:expr_2021) => {{
        let server = $crate::transports::tcp::TcpServer::builder($addr)
            .processor($processor)
            .build()?;
        server.run()
    }};
}

/// Create and run a TCP streaming JSON-RPC server
///
/// # Usage:
/// ```text
/// // Basic TCP streaming server
/// rpc_tcp_stream_server!("127.0.0.1:8080", registry).await?;
///
/// // With error handling
/// rpc_tcp_stream_server!("127.0.0.1:8080", registry).await.expect("Server failed");
/// ```
#[cfg(feature = "tcp-stream")]
#[macro_export]
macro_rules! rpc_tcp_stream_server {
    ($addr:expr_2021, $processor:expr_2021) => {
        async move {
            let server = $crate::transports::tcp_stream::TcpStreamServer::builder($addr)
                .processor($processor)
                .build()?;
            server.run().await
        }
    };
}

/// Create a TCP streaming JSON-RPC client
///
/// # Usage:
/// ```text
/// // Create and connect client
/// let mut client = rpc_tcp_stream_client!("127.0.0.1:8080").await?;
///
/// // Send request and get response
/// let response = client.call("method_name", Some(params)).await?;
/// ```
#[cfg(feature = "tcp-stream")]
#[macro_export]
macro_rules! rpc_tcp_stream_client {
    ($addr:expr_2021) => {{
        $crate::transports::tcp_stream::TcpStreamClient::builder($addr)
            .build()
            .connect()
    }};
}

// Stateful Macros - Easy stateful processor creation

/// Create a stateful JSON-RPC processor
///
/// # Usage:
/// ```text
/// // Create processor with context and handler
/// let processor = rpc_stateful_processor!(service_context, handler);
///
/// // Create processor with context and method registry
/// let processor = rpc_stateful_processor!(service_context, registry);
/// ```
#[cfg(feature = "stateful")]
#[macro_export]
macro_rules! rpc_stateful_processor {
    ($context:expr_2021, $handler:expr_2021) => {
        $crate::stateful::StatefulProcessor::new($context, $handler)
    };
}

/// Create a stateful method registry
///
/// # Usage:
/// ```text
/// // Create empty registry
/// let registry: StatefulMethodRegistry<MyContext> = rpc_stateful_registry!();
///
/// // Add methods with builder pattern
/// let registry = rpc_stateful_registry!()
///     .register_fn("method1", handler1)
///     .register_fn("method2", handler2);
/// ```
#[cfg(feature = "stateful")]
#[macro_export]
macro_rules! rpc_stateful_registry {
    () => {
        $crate::stateful::StatefulMethodRegistry::new()
    };
}

/// Create a stateful processor with builder pattern
///
/// # Usage:
/// ```text
/// // Create processor with builder
/// let processor = rpc_stateful_builder!(context)
///     .handler(handler)
///     .build()?;
///
/// // Create processor with registry
/// let processor = rpc_stateful_builder!(context)
///     .registry(registry)
///     .build()?;
/// ```
#[cfg(feature = "stateful")]
#[macro_export]
macro_rules! rpc_stateful_builder {
    ($context:expr_2021) => {
        $crate::stateful::StatefulProcessor::builder($context)
    };
}

//
// Method Definition Macros - Easy method creation
//

/// Define a simple JSON-RPC method with automatic trait implementation
///
/// # Usage:
/// ```text
/// rpc_method!(PingMethod, "ping", |_params, id| {
///     rpc_success!("pong", id)
/// });
///
/// rpc_method!(AddMethod, "add", |params, id| {
///     let nums: Vec<i32> = serde_json::from_value(params.unwrap_or_default()).unwrap();
///     rpc_success!(nums.iter().sum::<i32>(), id)
/// });
/// ```
#[macro_export]
macro_rules! rpc_method {
    ($name:ident, $method_name:expr, $handler:expr) => {
        pub struct $name;

        #[async_trait::async_trait]
        impl $crate::JsonRPCMethod for $name {
            fn method_name(&self) -> &'static str {
                $method_name
            }

            async fn call(
                &self,
                params: Option<serde_json::Value>,
                id: Option<$crate::RequestId>,
            ) -> $crate::Response {
                ($handler)(params, id)
            }
        }
    };
}

/// Validate and extract parameters with automatic error responses
///
/// # Usage:
/// ```text
/// rpc_method!(AddMethod, "add", |params, id| {
///     let numbers = rpc_params!(params, id => Vec<i32>);
///     rpc_success!(numbers.iter().sum::<i32>(), id)
/// });
/// ```
#[macro_export]
macro_rules! rpc_params {
    ($params:expr, $id:expr => $type:ty) => {
        match $params {
            Some(p) => match serde_json::from_value::<$type>(p) {
                Ok(params) => params,
                Err(_) => return $crate::rpc_invalid_params!("Invalid parameter format", $id),
            },
            None => return $crate::rpc_invalid_params!("Missing required parameters", $id),
        }
    };
    ($params:expr, $id:expr => Option<$type:ty>) => {
        match $params {
            Some(p) => match serde_json::from_value::<$type>(p) {
                Ok(params) => Some(params),
                Err(_) => return $crate::rpc_invalid_params!("Invalid parameter format", $id),
            },
            None => None,
        }
    };
}

/// Convert Result types to JSON-RPC responses with error logging
///
/// This macro logs detailed errors server-side and returns a generic error.
/// For custom error messages, provide them explicitly.
///
/// # Usage:
/// ```text
/// rpc_method!(DivideMethod, "divide", |params, id| {
///     let [a, b]: [f64; 2] = rpc_params!(params, id => [f64; 2]);
///     let result = if b != 0.0 { Ok(a / b) } else { Err("Division by zero") };
///     rpc_try!(result, id)
/// });
/// ```
#[macro_export]
macro_rules! rpc_try {
    ($result:expr, $id:expr) => {
        match $result {
            Ok(value) => $crate::rpc_success!(value, $id),
            Err(error) => {
                tracing::error!(
                    error = %error,
                    request_id = ?$id,
                    "method execution failed"
                );
                $crate::rpc_error!(
                    $crate::error_codes::INTERNAL_ERROR,
                    "Internal server error",
                    $id
                )
            },
        }
    };
    ($result:expr, $id:expr, $error_code:expr) => {
        match $result {
            Ok(value) => $crate::rpc_success!(value, $id),
            Err(error) => {
                tracing::error!(
                    error = %error,
                    request_id = ?$id,
                    error_code = $error_code,
                    "method execution failed"
                );
                $crate::rpc_error!(
                    $error_code,
                    "Server error",
                    $id
                )
            },
        }
    };
    ($result:expr, $id:expr, $error_code:expr, $message:expr) => {
        match $result {
            Ok(value) => $crate::rpc_success!(value, $id),
            Err(error) => {
                tracing::error!(
                    error = %error,
                    request_id = ?$id,
                    error_code = $error_code,
                    "method execution failed"
                );
                $crate::rpc_error!($error_code, $message, $id)
            },
        }
    };
}

/// Extract result from JSON-RPC response
///
/// # Usage:
/// ```text
/// let value = rpc_extract!(response);
/// let typed_value: i32 = rpc_extract!(response => i32);
/// ```
#[macro_export]
macro_rules! rpc_extract {
    ($response:expr) => {
        $response
            .result()
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Null)
    };
    ($response:expr => $type:ty) => {
        serde_json::from_value::<$type>($crate::rpc_extract!($response)).unwrap_or_default()
    };
}

/// Build a registry with multiple method instances
///
/// # Usage:
/// ```text
/// let registry = rpc_registry_with_methods![PingMethod, EchoMethod, AddMethod];
/// ```
#[macro_export]
macro_rules! rpc_registry_with_methods {
    ($($method:expr),* $(,)?) => {
        $crate::MethodRegistry::new($crate::register_methods![$($method),*])
    };
}

/// Create a simple JSON-RPC client call
///
/// # Usage:
/// ```text
/// let request = rpc_call_request!("method_name", [1, 2, 3], 42);
/// let request = rpc_call_request!("ping", 1); // no params
/// ```
#[macro_export]
macro_rules! rpc_call_request {
    ($method:expr, $params:expr, $id:expr) => {
        $crate::RequestBuilder::new($method)
            .params(serde_json::json!($params))
            .id(serde_json::json!($id))
            .build()
    };
    ($method:expr, $id:expr) => {
        $crate::RequestBuilder::new($method)
            .id(serde_json::json!($id))
            .build()
    };
}

/// Handle common validation patterns
///
/// # Usage:
/// ```text
/// rpc_validate!(value > 0, "Value must be positive", id);
/// rpc_validate!(!name.is_empty(), "Name cannot be empty", id);
/// ```
#[macro_export]
macro_rules! rpc_validate {
    ($condition:expr, $message:expr, $id:expr) => {
        if !($condition) {
            return $crate::rpc_invalid_params!($message, $id);
        }
    };
}
