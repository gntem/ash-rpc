use ash_rpc::stateful::{ServiceContext, StatefulMethodRegistry, StatefulProcessor};
use ash_rpc::{MessageProcessor, ResponseBuilder};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct AppError(String);

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AppError {}

struct DatabaseContext {
    data: Arc<Mutex<HashMap<String, serde_json::Value>>>,
    config: AppConfig,
}

struct AppConfig {
    max_entries: usize,
    allow_overwrite: bool,
}

impl ServiceContext for DatabaseContext {
    type Error = AppError;
}

impl DatabaseContext {
    fn new(config: AppConfig) -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    fn get(&self, key: &str) -> Result<Option<serde_json::Value>, AppError> {
        let data = self
            .data
            .lock()
            .map_err(|e| AppError(format!("Lock error: {e}")))?;
        Ok(data.get(key).cloned())
    }

    fn set(&self, key: String, value: serde_json::Value) -> Result<(), AppError> {
        let mut data = self
            .data
            .lock()
            .map_err(|e| AppError(format!("Lock error: {e}")))?;

        if data.len() >= self.config.max_entries && !data.contains_key(&key) {
            return Err(AppError("Database full".to_string()));
        }

        if data.contains_key(&key) && !self.config.allow_overwrite {
            return Err(AppError(
                "Key already exists and overwrite not allowed".to_string(),
            ));
        }

        data.insert(key, value);
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<bool, AppError> {
        let mut data = self
            .data
            .lock()
            .map_err(|e| AppError(format!("Lock error: {e}")))?;
        Ok(data.remove(key).is_some())
    }

    fn list_keys(&self) -> Result<Vec<String>, AppError> {
        let data = self
            .data
            .lock()
            .map_err(|e| AppError(format!("Lock error: {e}")))?;
        Ok(data.keys().cloned().collect())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig {
        max_entries: 100,
        allow_overwrite: true,
    };

    let context = DatabaseContext::new(config);

    let registry = StatefulMethodRegistry::new()
        .register_fn("get", |ctx: &DatabaseContext, params, id| {
            let params = params.ok_or_else(|| AppError("Missing parameters".to_string()))?;
            let key = params
                .get("key")
                .and_then(|k| k.as_str())
                .ok_or_else(|| AppError("Missing or invalid 'key' parameter".to_string()))?;

            match ctx.get(key)? {
                Some(value) => Ok(ResponseBuilder::new()
                    .success(serde_json::json!({
                        "found": true,
                        "value": value
                    }))
                    .id(id)
                    .build()),
                None => Ok(ResponseBuilder::new()
                    .success(serde_json::json!({
                        "found": false
                    }))
                    .id(id)
                    .build()),
            }
        })
        .register_fn("set", |ctx: &DatabaseContext, params, id| {
            let params = params.ok_or_else(|| AppError("Missing parameters".to_string()))?;
            let key = params
                .get("key")
                .and_then(|k| k.as_str())
                .ok_or_else(|| AppError("Missing or invalid 'key' parameter".to_string()))?;
            let value = params
                .get("value")
                .ok_or_else(|| AppError("Missing 'value' parameter".to_string()))?;

            ctx.set(key.to_string(), value.clone())?;

            Ok(ResponseBuilder::new()
                .success(serde_json::json!({
                    "success": true,
                    "message": "Value stored successfully"
                }))
                .id(id)
                .build())
        })
        .register_fn("delete", |ctx: &DatabaseContext, params, id| {
            let params = params.ok_or_else(|| AppError("Missing parameters".to_string()))?;
            let key = params
                .get("key")
                .and_then(|k| k.as_str())
                .ok_or_else(|| AppError("Missing or invalid 'key' parameter".to_string()))?;

            let deleted = ctx.delete(key)?;

            Ok(ResponseBuilder::new()
                .success(serde_json::json!({
                    "deleted": deleted
                }))
                .id(id)
                .build())
        })
        .register_fn("list", |ctx: &DatabaseContext, _params, id| {
            let keys = ctx.list_keys()?;

            Ok(ResponseBuilder::new()
                .success(serde_json::json!({
                    "keys": keys
                }))
                .id(id)
                .build())
        });

    let processor = StatefulProcessor::builder(context)
        .registry(registry)
        .build()?;

    println!("Created stateful processor with database context");
    println!("Available methods: get, set, delete, list");

    let request = ash_rpc::RequestBuilder::new("set")
        .params(serde_json::json!({
            "key": "test",
            "value": "Hello, World!"
        }))
        .id(serde_json::json!(1))
        .build();

    if let Some(response) = processor.process_message(ash_rpc::Message::Request(request)) {
        println!("Response: {}", serde_json::to_string_pretty(&response)?);
    }

    Ok(())
}
