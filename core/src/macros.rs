//! Convenience macros for JSON-RPC response creation.

/// Create a success response with a result value and optional ID
///
/// # Examples:
/// ```ignore
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
/// ```ignore
/// // Error with explicit code, message and ID
/// rpc_error!(-32602, "Invalid parameters", Some(1))
///
/// // Error with code and message, ID from variable
/// rpc_error!(-32602, "Invalid parameters", id)
///
/// // Error without ID
/// rpc_error!(-32601, "Method not found")
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
/// ```ignore
/// // Error with data
/// rpc_error_with_data!(-32602, "Invalid parameters", {"expected": "array"}, Some(1))
///
/// // Error with data but no ID
/// rpc_error_with_data!(-32602, "Invalid parameters", {"expected": "array"})
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
/// ```ignore
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
/// ```ignore
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
/// ```ignore
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
/// ```ignore
/// // Error with code and message
/// rpc_error_obj!(-32602, "Invalid parameters")
///
/// // Error with code, message and data
/// rpc_error_obj!(-32602, "Invalid parameters", {"expected": "array"})
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
/// ```ignore
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
        let server = $crate::transport::tcp::TcpServer::builder($addr)
            .processor($processor)
            .build()?;
        server.run()
    }};
}

/// Create and run a TCP streaming JSON-RPC server
///
/// # Usage:
/// ```ignore
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
            let server = $crate::transport::tcp_stream::TcpStreamServer::builder($addr)
                .processor($processor)
                .build()?;
            server.run().await
        }
    };
}

/// Create a TCP streaming JSON-RPC client
///
/// # Usage:
/// ```ignore
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
        $crate::transport::tcp_stream::TcpStreamClient::builder($addr)
            .build()
            .connect()
    }};
}

/// Create an Axum router with JSON-RPC endpoint - MOVED TO ash-rpc-contrib
///
/// # Usage:
/// ```ignore
/// // Basic router with default /rpc path
/// let app = rpc_axum_router!(registry);
//
// Stateful Macros - Easy stateful processor creation
//

/// Create a stateful JSON-RPC processor
///
/// # Usage:
/// ```ignore
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
/// ```ignore
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
/// ```ignore
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
