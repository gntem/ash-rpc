//! Contributed JSON-RPC methods and utilities for ash-rpc

#[cfg(feature = "healthcheck")]
use ash_rpc_core::{MethodRegistry, RequestId, Response, ResponseBuilder};

/// Health check method that returns "ok"
#[cfg(feature = "healthcheck")]
pub fn healthcheck_method(params: Option<serde_json::Value>, id: Option<RequestId>) -> Response {
    // Health check doesn't need parameters, but we'll ignore them if provided
    let _ = params;

    ResponseBuilder::new()
        .success(serde_json::json!("ok"))
        .id(id)
        .build()
}

/// Register the healthcheck method with a registry
#[cfg(feature = "healthcheck")]
pub fn register_healthcheck(registry: MethodRegistry) -> MethodRegistry {
    registry.register("healthcheck", healthcheck_method)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "healthcheck")]
    use ash_rpc_core::{Message, MessageProcessor, RequestBuilder};

    #[cfg(feature = "healthcheck")]
    #[test]
    fn test_healthcheck_method() {
        use super::*;

        let request = ash_rpc_core::RequestBuilder::new("healthcheck")
            .id(serde_json::json!(1))
            .build();

        let response = healthcheck_method(request.params, request.id);

        assert!(response.is_success());
        assert_eq!(response.result().unwrap(), &serde_json::json!("ok"));
        assert_eq!(response.id().unwrap(), &serde_json::json!(1));
    }

    #[cfg(feature = "healthcheck")]
    #[test]
    fn test_healthcheck_with_params() {
        use super::*;

        let params = serde_json::json!({"ignore": "this"});
        let request = RequestBuilder::new("healthcheck")
            .params(params.clone())
            .id(serde_json::json!(2))
            .build();

        let response = healthcheck_method(request.params, request.id);

        assert!(response.is_success());
        assert_eq!(response.result().unwrap(), &serde_json::json!("ok"));
        assert_eq!(response.id().unwrap(), &serde_json::json!(2));
    }

    #[cfg(feature = "healthcheck")]
    #[test]
    fn test_register_healthcheck() {
        use super::*;

        let registry = ash_rpc_core::MethodRegistry::new();
        let registry_with_healthcheck = register_healthcheck(registry);

        let request = RequestBuilder::new("healthcheck")
            .id(serde_json::json!(3))
            .build();

        let message = Message::Request(request);
        let response = registry_with_healthcheck.process_message(message).unwrap();

        assert!(response.is_success());
        assert_eq!(response.result().unwrap(), &serde_json::json!("ok"));
    }
}
