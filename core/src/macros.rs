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
    ($result:expr, $id:expr) => {
        $crate::ResponseBuilder::new()
            .success(serde_json::json!($result))
            .id($id)
            .build()
    };
    ($result:expr) => {
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
    ($code:expr, $message:expr, $id:expr) => {
        $crate::ResponseBuilder::new()
            .error($crate::ErrorBuilder::new($code, $message).build())
            .id($id)
            .build()
    };
    ($code:expr, $message:expr) => {
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
    ($code:expr, $message:expr, $data:expr, $id:expr) => {
        $crate::ResponseBuilder::new()
            .error(
                $crate::ErrorBuilder::new($code, $message)
                    .data(serde_json::json!($data))
                    .build()
            )
            .id($id)
            .build()
    };
    ($code:expr, $message:expr, $data:expr) => {
        $crate::ResponseBuilder::new()
            .error(
                $crate::ErrorBuilder::new($code, $message)
                    .data(serde_json::json!($data))
                    .build()
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
    ($message:expr, $id:expr) => {
        rpc_error!($crate::error_codes::INVALID_PARAMS, $message, $id)
    };
    ($message:expr) => {
        rpc_error!($crate::error_codes::INVALID_PARAMS, $message)
    };
}

#[macro_export]
macro_rules! rpc_method_not_found {
    ($id:expr) => {
        rpc_error!($crate::error_codes::METHOD_NOT_FOUND, "Method not found", $id)
    };
    () => {
        rpc_error!($crate::error_codes::METHOD_NOT_FOUND, "Method not found")
    };
}

#[macro_export]
macro_rules! rpc_parse_error {
    ($message:expr, $id:expr) => {
        rpc_error!($crate::error_codes::PARSE_ERROR, $message, $id)
    };
    ($message:expr) => {
        rpc_error!($crate::error_codes::PARSE_ERROR, $message)
    };
}

#[macro_export]
macro_rules! rpc_internal_error {
    ($message:expr, $id:expr) => {
        rpc_error!($crate::error_codes::INTERNAL_ERROR, $message, $id)
    };
    ($message:expr) => {
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
    ($method:expr) => {
        $crate::RequestBuilder::new($method)
            .build()
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
    ($method:expr, $params:expr) => {
        $crate::NotificationBuilder::new($method)
            .params(serde_json::json!($params))
            .build()
    };
    ($method:expr) => {
        $crate::NotificationBuilder::new($method)
            .build()
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
    ($code:expr, $message:expr, $data:expr) => {
        $crate::ErrorBuilder::new($code, $message)
            .data(serde_json::json!($data))
            .build()
    };
    ($code:expr, $message:expr) => {
        $crate::ErrorBuilder::new($code, $message)
            .build()
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
    ($addr:expr, $processor:expr) => {{
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
    ($addr:expr, $processor:expr) => {
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
    ($addr:expr) => {
        {
            $crate::transport::tcp_stream::TcpStreamClient::builder($addr)
                .build()
                .connect()
        }
    };
}

/// Create an Axum router with JSON-RPC endpoint
/// 
/// # Usage:
/// ```ignore
/// // Basic router with default /rpc path
/// let app = rpc_axum_router!(registry);
/// 
/// // Router with custom path
/// let app = rpc_axum_router!(registry, "/api/rpc");
/// 
/// // Router with additional routes
/// let app = rpc_axum_router!(registry, "/rpc")
///     .route("/health", get(health_check));
/// ```
#[cfg(feature = "axum")]
#[macro_export]
macro_rules! rpc_axum_router {
    ($processor:expr, $path:expr) => {
        $crate::transport::axum::create_rpc_router($processor, $path)
    };
    ($processor:expr) => {
        $crate::transport::axum::create_rpc_router($processor, "/rpc")
    };
}

/// Create and run an Axum server with JSON-RPC
/// 
/// # Usage:
/// ```ignore
/// // Basic server on default port
/// rpc_axum_server!("127.0.0.1:3000", registry).await?;
/// 
/// // Server with custom RPC path
/// rpc_axum_server!("127.0.0.1:3000", registry, "/api/rpc").await?;
/// ```
#[cfg(feature = "axum")]
#[macro_export]
macro_rules! rpc_axum_server {
    ($addr:expr, $processor:expr, $path:expr) => {
        {
            let app = rpc_axum_router!($processor, $path);
            async move {
                let listener = tokio::net::TcpListener::bind($addr).await?;
                axum::serve(listener, app).await
            }
        }
    };
    ($addr:expr, $processor:expr) => {
        {
            let app = rpc_axum_router!($processor);
            async move {
                let listener = tokio::net::TcpListener::bind($addr).await?;
                axum::serve(listener, app).await
            }
        }
    };
}

/// Create an Axum RPC layer for middleware use
/// 
/// # Usage:
/// ```ignore
/// // Create RPC layer
/// let rpc_layer = rpc_axum_layer!(registry);
/// 
/// // Use in router
/// let app = Router::new()
///     .route("/health", get(health_check))
///     .layer(rpc_layer);
/// ```
#[cfg(feature = "axum")]
#[macro_export]
macro_rules! rpc_axum_layer {
    ($processor:expr, $path:expr) => {
        $crate::transport::axum::AxumRpcLayer::builder()
            .processor($processor)
            .path($path)
            .build()
            .expect("Failed to build Axum RPC layer")
    };
    ($processor:expr) => {
        $crate::transport::axum::AxumRpcLayer::builder()
            .processor($processor)
            .build()
            .expect("Failed to build Axum RPC layer")
    };
}
