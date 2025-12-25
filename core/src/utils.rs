//! Utility functions for JSON-RPC documentation generation.

use crate::traits::MethodInfo;
use std::collections::HashMap;

/// Generate OpenAPI/Swagger documentation for JSON-RPC methods
pub fn render_docs(method_info: &HashMap<String, MethodInfo>) -> serde_json::Value {
    let mut methods = serde_json::Map::new();

    for (method_name, info) in method_info {
        let mut method_doc = serde_json::Map::new();

        // Basic method info
        method_doc.insert(
            "summary".to_string(),
            serde_json::Value::String(
                info.description
                    .clone()
                    .unwrap_or_else(|| format!("Execute {} method", method_name)),
            ),
        );

        // Request body schema
        let mut request_body = serde_json::Map::new();
        let mut content = serde_json::Map::new();
        let mut json_content = serde_json::Map::new();
        let mut schema = serde_json::Map::new();

        schema.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );

        let mut properties = serde_json::Map::new();
        properties.insert(
            "jsonrpc".to_string(),
            serde_json::json!({
                "type": "string",
                "enum": ["2.0"],
                "description": "JSON-RPC version"
            }),
        );
        properties.insert(
            "method".to_string(),
            serde_json::json!({
                "type": "string",
                "enum": [method_name],
                "description": "Method name"
            }),
        );
        properties.insert(
            "id".to_string(),
            serde_json::json!({
                "oneOf": [
                    {"type": "string"},
                    {"type": "number"},
                    {"type": "null"}
                ],
                "description": "Request identifier"
            }),
        );

        if let Some(params_schema) = &info.params_schema {
            properties.insert("params".to_string(), params_schema.clone());
        }

        schema.insert(
            "properties".to_string(),
            serde_json::Value::Object(properties),
        );
        schema.insert(
            "required".to_string(),
            serde_json::json!(["jsonrpc", "method"]),
        );

        json_content.insert("schema".to_string(), serde_json::Value::Object(schema));
        content.insert(
            "application/json".to_string(),
            serde_json::Value::Object(json_content),
        );
        request_body.insert("content".to_string(), serde_json::Value::Object(content));
        request_body.insert("required".to_string(), serde_json::Value::Bool(true));

        method_doc.insert(
            "requestBody".to_string(),
            serde_json::Value::Object(request_body),
        );

        // Response schema
        let mut responses = serde_json::Map::new();
        let mut success_response = serde_json::Map::new();
        success_response.insert(
            "description".to_string(),
            serde_json::Value::String("Successful response".to_string()),
        );

        let mut success_content = serde_json::Map::new();
        let mut success_json = serde_json::Map::new();
        let mut success_schema = serde_json::Map::new();

        success_schema.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );

        let mut success_properties = serde_json::Map::new();
        success_properties.insert(
            "jsonrpc".to_string(),
            serde_json::json!({
                "type": "string",
                "enum": ["2.0"]
            }),
        );
        success_properties.insert(
            "id".to_string(),
            serde_json::json!({
                "oneOf": [
                    {"type": "string"},
                    {"type": "number"},
                    {"type": "null"}
                ]
            }),
        );

        if let Some(result_schema) = &info.result_schema {
            success_properties.insert("result".to_string(), result_schema.clone());
        } else {
            success_properties.insert(
                "result".to_string(),
                serde_json::json!({
                    "description": "Method result"
                }),
            );
        }

        success_schema.insert(
            "properties".to_string(),
            serde_json::Value::Object(success_properties),
        );
        success_schema.insert(
            "required".to_string(),
            serde_json::json!(["jsonrpc", "result"]),
        );

        success_json.insert(
            "schema".to_string(),
            serde_json::Value::Object(success_schema),
        );
        success_content.insert(
            "application/json".to_string(),
            serde_json::Value::Object(success_json),
        );
        success_response.insert(
            "content".to_string(),
            serde_json::Value::Object(success_content),
        );

        responses.insert(
            "200".to_string(),
            serde_json::Value::Object(success_response),
        );

        // Error response
        let error_response = serde_json::json!({
            "description": "Error response",
            "content": {
                "application/json": {
                    "schema": {
                        "type": "object",
                        "properties": {
                            "jsonrpc": {
                                "type": "string",
                                "enum": ["2.0"]
                            },
                            "error": {
                                "type": "object",
                                "properties": {
                                    "code": {
                                        "type": "integer",
                                        "description": "Error code"
                                    },
                                    "message": {
                                        "type": "string",
                                        "description": "Error message"
                                    },
                                    "data": {
                                        "description": "Additional error data"
                                    }
                                },
                                "required": ["code", "message"]
                            },
                            "id": {
                                "oneOf": [
                                    {"type": "string"},
                                    {"type": "number"},
                                    {"type": "null"}
                                ]
                            }
                        },
                        "required": ["jsonrpc", "error"]
                    }
                }
            }
        });

        responses.insert("400".to_string(), error_response);
        method_doc.insert(
            "responses".to_string(),
            serde_json::Value::Object(responses),
        );

        methods.insert(method_name.clone(), serde_json::Value::Object(method_doc));
    }

    // Build the full OpenAPI document
    serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": "JSON-RPC API",
            "description": "Auto-generated documentation for JSON-RPC methods",
            "version": "1.0.0"
        },
        "servers": [
            {
                "url": "/",
                "description": "JSON-RPC endpoint"
            }
        ],
        "paths": {
            "/": {
                "post": {
                    "summary": "JSON-RPC endpoint",
                    "description": "Execute JSON-RPC methods",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "oneOf": methods.values().map(|method| {
                                        if let serde_json::Value::Object(method_obj) = method
                                            && let Some(serde_json::Value::Object(request_body)) = method_obj.get("requestBody")
                                            && let Some(serde_json::Value::Object(content)) = request_body.get("content")
                                            && let Some(serde_json::Value::Object(json_content)) = content.get("application/json")
                                        {
                                            return json_content.get("schema").cloned().unwrap_or(serde_json::Value::Null);
                                        }
                                        serde_json::Value::Null
                                    }).collect::<Vec<_>>()
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Successful response",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "oneOf": [
                                            {
                                                "type": "object",
                                                "properties": {
                                                    "jsonrpc": {"type": "string", "enum": ["2.0"]},
                                                    "result": {"description": "Method result"},
                                                    "id": {"oneOf": [{"type": "string"}, {"type": "number"}, {"type": "null"}]}
                                                },
                                                "required": ["jsonrpc", "result"]
                                            },
                                            {
                                                "type": "object",
                                                "properties": {
                                                    "jsonrpc": {"type": "string", "enum": ["2.0"]},
                                                    "error": {
                                                        "type": "object",
                                                        "properties": {
                                                            "code": {"type": "integer"},
                                                            "message": {"type": "string"},
                                                            "data": {"description": "Additional error data"}
                                                        },
                                                        "required": ["code", "message"]
                                                    },
                                                    "id": {"oneOf": [{"type": "string"}, {"type": "number"}, {"type": "null"}]}
                                                },
                                                "required": ["jsonrpc", "error"]
                                            }
                                        ]
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "JsonRpcRequest": {
                    "type": "object",
                    "properties": {
                        "jsonrpc": {"type": "string", "enum": ["2.0"]},
                        "method": {"type": "string"},
                        "params": {"description": "Method parameters"},
                        "id": {"oneOf": [{"type": "string"}, {"type": "number"}, {"type": "null"}]}
                    },
                    "required": ["jsonrpc", "method"]
                },
                "JsonRpcSuccessResponse": {
                    "type": "object",
                    "properties": {
                        "jsonrpc": {"type": "string", "enum": ["2.0"]},
                        "result": {"description": "Method result"},
                        "id": {"oneOf": [{"type": "string"}, {"type": "number"}, {"type": "null"}]}
                    },
                    "required": ["jsonrpc", "result"]
                },
                "JsonRpcErrorResponse": {
                    "type": "object",
                    "properties": {
                        "jsonrpc": {"type": "string", "enum": ["2.0"]},
                        "error": {
                            "type": "object",
                            "properties": {
                                "code": {"type": "integer"},
                                "message": {"type": "string"},
                                "data": {"description": "Additional error data"}
                            },
                            "required": ["code", "message"]
                        },
                        "id": {"oneOf": [{"type": "string"}, {"type": "number"}, {"type": "null"}]}
                    },
                    "required": ["jsonrpc", "error"]
                }
            }
        },
        "tags": [
            {
                "name": "JSON-RPC",
                "description": "JSON-RPC 2.0 methods"
            }
        ]
    })
}
