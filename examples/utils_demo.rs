use ash_rpc_core::{utils, MethodInfo};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("=== Utils Standalone Function Demo ===");

    let mut method_info = HashMap::new();

    method_info.insert(
        "hello".to_string(),
        MethodInfo::new("hello")
            .with_description("Say hello to someone")
            .with_params_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the person to greet"
                    }
                },
                "required": ["name"]
            }))
            .with_result_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "greeting": {
                        "type": "string",
                        "description": "The greeting message"
                    }
                },
                "required": ["greeting"]
            })),
    );

    method_info.insert(
        "goodbye".to_string(),
        MethodInfo::new("goodbye")
            .with_description("Say goodbye to someone")
            .with_params_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the person to say goodbye to"
                    }
                },
                "required": ["name"]
            })),
    );

    println!("Using utils::render_docs() directly...");
    let docs = utils::render_docs(&method_info);

    println!("Generated documentation for {} methods", method_info.len());

    if let Some(info) = docs.get("info") {
        println!(
            "API Title: {}",
            info.get("title").unwrap_or(&serde_json::Value::Null)
        );
        println!(
            "API Version: {}",
            info.get("version").unwrap_or(&serde_json::Value::Null)
        );
    }

    if let Some(paths) = docs.get("paths") {
        if let Some(_root) = paths.get("/") {
            println!("Endpoint: / (POST)");
        }
    }

    println!("\n=== Full Documentation ===");
    println!("{}", serde_json::to_string_pretty(&docs)?);

    Ok(())
}
