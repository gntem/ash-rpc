#[cfg(feature = "axum")]
mod example {
    use ash_rpc_core::transport::axum::AxumRpcLayer;
    use ash_rpc_stateful::{ServiceContext, StatefulMethodRegistry, StatefulProcessor};
    use ash_rpc_core::{ResponseBuilder, ErrorBuilder};
    use axum::Router;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug)]
    struct ServiceError(String);

    impl std::fmt::Display for ServiceError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for ServiceError {}

    #[derive(Clone)]
    struct SessionData {
        user_id: String,
        created_at: u64,
        last_active: u64,
    }

    struct SessionService {
        sessions: Arc<RwLock<HashMap<String, SessionData>>>,
        session_timeout: u64,
    }

    impl ServiceContext for SessionService {
        type Error = ServiceError;
    }

    impl SessionService {
        fn new(session_timeout: u64) -> Self {
            Self {
                sessions: Arc::new(RwLock::new(HashMap::new())),
                session_timeout,
            }
        }

        fn create_session(&self, user_id: String) -> Result<String, ServiceError> {
            let session_id = format!("sess_{}", uuid::Uuid::new_v4().simple());
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| ServiceError(format!("Time error: {}", e)))?
                .as_secs();

            let session = SessionData {
                user_id,
                created_at: now,
                last_active: now,
            };

            let mut sessions = self.sessions.write()
                .map_err(|e| ServiceError(format!("Lock error: {}", e)))?;
            sessions.insert(session_id.clone(), session);

            Ok(session_id)
        }

        fn get_session(&self, session_id: &str) -> Result<Option<SessionData>, ServiceError> {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| ServiceError(format!("Time error: {}", e)))?
                .as_secs();

            let sessions = self.sessions.read()
                .map_err(|e| ServiceError(format!("Lock error: {}", e)))?;

            if let Some(session) = sessions.get(session_id) {
                if now - session.last_active > self.session_timeout {
                    return Ok(None);
                }
                Ok(Some(session.clone()))
            } else {
                Ok(None)
            }
        }

        fn update_session(&self, session_id: &str) -> Result<bool, ServiceError> {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| ServiceError(format!("Time error: {}", e)))?
                .as_secs();

            let mut sessions = self.sessions.write()
                .map_err(|e| ServiceError(format!("Lock error: {}", e)))?;

            if let Some(session) = sessions.get_mut(session_id) {
                session.last_active = now;
                Ok(true)
            } else {
                Ok(false)
            }
        }

        fn delete_session(&self, session_id: &str) -> Result<bool, ServiceError> {
            let mut sessions = self.sessions.write()
                .map_err(|e| ServiceError(format!("Lock error: {}", e)))?;
            Ok(sessions.remove(session_id).is_some())
        }
    }

    fn create_session_registry() -> StatefulMethodRegistry<SessionService> {
        StatefulMethodRegistry::new()
            .register_fn("create_session", |ctx: &SessionService, params, id| {
                let user_id = params
                    .and_then(|p| p.get("user_id"))
                    .and_then(|u| u.as_str())
                    .ok_or_else(|| ServiceError("Missing user_id parameter".to_string()))?;

                match ctx.create_session(user_id.to_string()) {
                    Ok(session_id) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "session_id": session_id,
                            "user_id": user_id
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
            .register_fn("get_session", |ctx: &SessionService, params, id| {
                let session_id = params
                    .and_then(|p| p.get("session_id"))
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| ServiceError("Missing session_id parameter".to_string()))?;

                match ctx.get_session(session_id) {
                    Ok(Some(session)) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "valid": true,
                            "user_id": session.user_id,
                            "created_at": session.created_at,
                            "last_active": session.last_active
                        }))
                        .id(id)
                        .build()),
                    Ok(None) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "valid": false
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
            .register_fn("delete_session", |ctx: &SessionService, params, id| {
                let session_id = params
                    .and_then(|p| p.get("session_id"))
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| ServiceError("Missing session_id parameter".to_string()))?;

                match ctx.delete_session(session_id) {
                    Ok(deleted) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "deleted": deleted
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
    }

    pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
        let session_service = SessionService::new(3600); // 1 hour timeout
        let registry = create_session_registry();
        
        let processor = StatefulProcessor::builder(session_service)
            .registry(registry)
            .build()?;

        let rpc_layer = AxumRpcLayer::builder()
            .processor(processor)
            .path("/api/sessions")
            .build()?;

        let app = Router::new()
            .merge(rpc_layer.into_router());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3002").await?;
        println!("Stateful Session Axum server listening on http://127.0.0.1:3002/api/sessions");
        println!("Available methods: create_session, get_session, delete_session");
        
        axum::serve(listener, app).await?;
        Ok(())
    }
}

#[cfg(feature = "axum")]
fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(example::run_server())
        .unwrap();
}

#[cfg(not(feature = "axum"))]
fn main() {
    println!("This example requires the 'axum' feature to be enabled.");
    println!("Run with: cargo run --example axum_stateful_server --features axum");
}
