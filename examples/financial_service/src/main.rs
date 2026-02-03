//! Financial service example with audit logging and observability
//!
//! Demonstrates:
//! - TCP stateful server with SQLite database
//! - Audit logging for sensitive operations
//! - Basic tracing for general operations
//! - Healthcheck endpoint
//! - Financial data access tracking

use ash_rpc::audit_logging::{
    AuditBackend, AuditEvent, AuditEventType, AuditIntegrity, AuditProcessor, AuditResult,
    AuditSeverity, SequenceIntegrity, StdoutAuditBackend,
};
use ash_rpc::{
    stateful::{ServiceContext, StatefulJsonRPCMethod, StatefulMethodRegistry, StatefulProcessor},
    Error, Message, MessageProcessor, RequestId, Response,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Row};
use std::sync::Arc;
use tracing::{error, info, warn};

/// Custom error type for the service
#[derive(Debug)]
struct ServiceError(String);

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ServiceError {}

impl From<sqlx::Error> for ServiceError {
    fn from(err: sqlx::Error) -> Self {
        ServiceError(format!("Database error: {}", err))
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for ServiceError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        ServiceError(err.to_string())
    }
}

/// Application state shared across all connections
#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    audit_backend: Arc<dyn AuditBackend>,
    audit_integrity: Arc<dyn AuditIntegrity>,
}

impl ServiceContext for AppState {
    type Error = ServiceError;
}

/// Request parameters for get_financial_information
#[derive(Debug, Deserialize)]
struct GetFinancialInfoParams {
    accessor: String,
}

/// Response for financial information
#[derive(Debug, Serialize)]
struct FinancialInfoResponse {
    userid: String,
    data: String,
    accessed_by: String,
}

/// Financial information retrieval method
struct GetFinancialInfo;

#[async_trait]
impl StatefulJsonRPCMethod<AppState> for GetFinancialInfo {
    fn method_name(&self) -> &'static str {
        "get_financial_information"
    }

    async fn call(
        &self,
        context: &AppState,
        params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Result<Response, <AppState as ServiceContext>::Error> {
        info!("Processing financial information request");

        let params: GetFinancialInfoParams = match params {
            Some(p) => match serde_json::from_value(p) {
                Ok(params) => params,
                Err(e) => {
                    warn!("Invalid parameters: {}", e);
                    return Ok(Response::error(
                        ash_rpc::ErrorBuilder::new(ash_rpc::error_codes::INVALID_PARAMS, "Invalid params: expected {accessor: string}").build(),
                        id,
                    ));
                }
            },
            None => {
                warn!("Missing parameters");
                return Ok(Response::error(
                    ash_rpc::ErrorBuilder::new(ash_rpc::error_codes::INVALID_PARAMS, "Missing accessor parameter").build(),
                    id,
                ));
            }
        };

        // Query database for financial information
        let result = sqlx::query("SELECT userid, important_info FROM account WHERE userid = ?")
            .bind(&params.accessor)
            .fetch_optional(&context.db)
            .await;

        match result {
            Ok(Some(row)) => {
                let userid: String = row.get("userid");
                let important_info: String = row.get("important_info");

                // Log audit event for sensitive data access
                let mut audit_event = AuditEvent::builder()
                    .event_type(AuditEventType::MethodInvocation)
                    .method("get_financial_information")
                    .principal(&params.accessor)
                    .result(AuditResult::Success)
                    .severity(AuditSeverity::Critical)
                    .metadata("accessed_userid", userid.clone())
                    .metadata("data_type", "financial_information")
                    .build();

                context.audit_integrity.add_integrity(&mut audit_event);
                context.audit_backend.log_audit(&audit_event);

                info!("Financial data accessed: user={} by={}", userid, params.accessor);

                Ok(Response::success(
                    serde_json::json!(FinancialInfoResponse {
                        userid,
                        data: important_info,
                        accessed_by: params.accessor,
                    }),
                    id,
                ))
            }
            Ok(None) => {
                warn!("Account not found: {}", params.accessor);

                // Log failed access attempt
                let mut audit_event = AuditEvent::builder()
                    .event_type(AuditEventType::MethodInvocation)
                    .method("get_financial_information")
                    .principal(&params.accessor)
                    .result(AuditResult::Failure)
                    .severity(AuditSeverity::Warning)
                    .metadata("reason", "account_not_found")
                    .build();

                context.audit_integrity.add_integrity(&mut audit_event);
                context.audit_backend.log_audit(&audit_event);

                Ok(Response::error(ash_rpc::ErrorBuilder::new(ash_rpc::error_codes::INTERNAL_ERROR, "Account not found").build(), id))
            }
            Err(e) => {
                error!("Database error: {}", e);

                // Log database error
                let mut audit_event = AuditEvent::builder()
                    .event_type(AuditEventType::ErrorOccurred)
                    .method("get_financial_information")
                    .principal(&params.accessor)
                    .result(AuditResult::Failure)
                    .severity(AuditSeverity::Critical)
                    .error(&format!("Database error: {}", e))
                    .build();

                context.audit_integrity.add_integrity(&mut audit_event);
                context.audit_backend.log_audit(&audit_event);

                Ok(Response::error(ash_rpc::ErrorBuilder::new(ash_rpc::error_codes::INTERNAL_ERROR, "Internal server error").build(), id))
            }
        }
    }
}

/// Health check response
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    database: String,
}

/// Health check method
struct Health;

#[async_trait]
impl StatefulJsonRPCMethod<AppState> for Health {
    fn method_name(&self) -> &'static str {
        "health"
    }

    async fn call(
        &self,
        context: &AppState,
        _params: Option<serde_json::Value>,
        id: Option<RequestId>,
    ) -> Result<Response, <AppState as ServiceContext>::Error> {
        // Check database connectivity
        let db_status = match sqlx::query("SELECT 1").fetch_one(&context.db).await {
            Ok(_) => "connected",
            Err(_) => "disconnected",
        };

        let status = if db_status == "connected" {
            "healthy"
        } else {
            "unhealthy"
        };

        Ok(Response::success(
            serde_json::json!(HealthResponse {
                status: status.to_string(),
                database: db_status.to_string(),
            }),
            id,
        ))
    }
}

/// Initialize database and seed data
async fn init_database(db_url: &str) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    info!("Initializing database at {}", db_url);

    let pool = SqlitePool::connect(db_url).await?;

    // Create account table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS account (
            userid TEXT PRIMARY KEY,
            important_info TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await?;

    // Seed initial data
    let seed_data = vec![
        ("alice", "Account balance: $50,000 | Credit score: 780"),
        ("bob", "Account balance: $125,000 | Credit score: 820"),
        ("charlie", "Account balance: $8,500 | Credit score: 650"),
        ("dave", "Account balance: $250,000 | Credit score: 795"),
    ];

    for (userid, info) in seed_data {
        sqlx::query("INSERT OR IGNORE INTO account (userid, important_info) VALUES (?, ?)")
            .bind(userid)
            .bind(info)
            .execute(&pool)
            .await?;
    }

    info!("Database initialized with {} seed accounts", 4);

    Ok(pool)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting financial service...");

    // Initialize database
    let db_url = "sqlite:sample.db";
    let pool = init_database(db_url).await?;

    // Create audit infrastructure
    let audit_backend: Arc<dyn AuditBackend> = Arc::new(StdoutAuditBackend);
    let audit_integrity: Arc<dyn AuditIntegrity> = Arc::new(SequenceIntegrity::new());

    // Log service startup
    let mut startup_event = AuditEvent::builder()
        .event_type(AuditEventType::AdminAction)
        .method("service_startup")
        .result(AuditResult::Success)
        .severity(AuditSeverity::Info)
        .metadata("service", "financial_service")
        .build();
    audit_integrity.add_integrity(&mut startup_event);
    audit_backend.log_audit(&startup_event);

    // Create application state
    let state = AppState {
        db: pool.clone(),
        audit_backend: Arc::clone(&audit_backend),
        audit_integrity: Arc::clone(&audit_integrity),
    };

    // Create stateful processor with methods
    let registry = StatefulMethodRegistry::new()
        .register(GetFinancialInfo)
        .register(Health);
    let stateful_processor = StatefulProcessor::new(state, registry);

    // Wrap stateful processor with audit processor
    let processor: Arc<dyn MessageProcessor + Send + Sync> = Arc::new(stateful_processor);
    let audited_processor = Arc::new(AuditProcessor::builder(processor)
        .with_backend(audit_backend)
        .with_integrity(audit_integrity)
        .build());

    // Start TCP server
    let addr = "127.0.0.1:9001";
    info!("Financial service listening on {}", addr);
    info!("Available methods:");
    info!("  - get_financial_information(accessor: string)");
    info!("  - health() -> checks service and database status");
    info!("");
    info!("Seeded accounts: alice, bob, charlie, dave");
    info!("");
    info!("Example request:");
    info!(r#"  echo '{{"jsonrpc":"2.0","method":"get_financial_information","params":{{"accessor":"alice"}},"id":1}}' | nc localhost 9001"#);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Setup graceful shutdown signal handling
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

    // Spawn signal handler
    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to setup SIGTERM handler");
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("Failed to setup SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => info!("Received SIGTERM"),
            _ = sigint.recv() => info!("Received SIGINT"),
        }

        info!("Initiating graceful shutdown...");
        let _ = shutdown_tx.send(true);
    });

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((socket, remote_addr)) => {
                        info!("New connection from {}", remote_addr);

                        let processor = Arc::clone(&audited_processor);
                        let mut shutdown_signal = shutdown_rx.clone();

                        tokio::spawn(async move {
                            use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

                            let (reader, mut writer) = socket.into_split();
                            let mut reader = BufReader::new(reader);
                            let mut line = String::new();

                            loop {
                                tokio::select! {
                                    read_result = reader.read_line(&mut line) => {
                                        match read_result {
                                            Ok(0) => break,
                                            Ok(_) => {
                                                let trimmed = line.trim();
                                                if trimmed.is_empty() {
                                                    line.clear();
                                                    continue;
                                                }

                                                match serde_json::from_str::<Message>(trimmed) {
                                                    Ok(message) => {
                                                        if let Some(response) = processor.process_message(message).await {
                                                            if let Ok(json) = serde_json::to_string(&response) {
                                                                let _ = writer.write_all(json.as_bytes()).await;
                                                                let _ = writer.write_all(b"\n").await;
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        warn!("Failed to parse message: {}", e);
                                                        let error_response = Response::error(
                                                            ash_rpc::ErrorBuilder::new(ash_rpc::error_codes::PARSE_ERROR, "Parse error").build(),
                                                            None,
                                                        );
                                                        if let Ok(json) = serde_json::to_string(&error_response) {
                                                            let _ = writer.write_all(json.as_bytes()).await;
                                                            let _ = writer.write_all(b"\n").await;
                                                        }
                                                    }
                                                }

                                                line.clear();
                                            }
                                            Err(e) => {
                                                warn!("Read error: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                    _ = shutdown_signal.changed() => {
                                        if *shutdown_signal.borrow() {
                                            info!("Connection handler shutting down: {}", remote_addr);
                                            break;
                                        }
                                    }
                                }
                            }

                            info!("Connection closed: {}", remote_addr);
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    break;
                }
            }
        }
    }

    info!("Server shutdown complete");
    Ok(())
}
