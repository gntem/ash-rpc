#[cfg(feature = "tcp")]
mod example {
    use ash_rpc_core::transport::tcp::TcpServer;
    use ash_rpc_core::{ErrorBuilder, ResponseBuilder};
    use ash_rpc_stateful::{ServiceContext, StatefulMethodRegistry, StatefulProcessor};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    pub struct ServiceError(String);

    impl std::fmt::Display for ServiceError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for ServiceError {}

    pub struct CounterService {
        counters: Arc<Mutex<HashMap<String, i64>>>,
        max_value: i64,
    }

    impl ServiceContext for CounterService {
        type Error = ServiceError;
    }

    impl CounterService {
        pub fn new(max_value: i64) -> Self {
            Self {
                counters: Arc::new(Mutex::new(HashMap::new())),
                max_value,
            }
        }

        pub fn increment(&self, name: &str) -> Result<i64, ServiceError> {
            let mut counters = self
                .counters
                .lock()
                .map_err(|e| ServiceError(format!("Lock error: {e}")))?;

            let current = counters.get(name).unwrap_or(&0);
            if *current >= self.max_value {
                return Err(ServiceError(format!(
                    "Counter {name} has reached maximum value {}",
                    self.max_value
                )));
            }

            let new_value = current + 1;
            counters.insert(name.to_string(), new_value);
            Ok(new_value)
        }

        pub fn get(&self, name: &str) -> Result<i64, ServiceError> {
            let counters = self
                .counters
                .lock()
                .map_err(|e| ServiceError(format!("Lock error: {e}")))?;
            Ok(*counters.get(name).unwrap_or(&0))
        }

        pub fn reset(&self, name: &str) -> Result<i64, ServiceError> {
            let mut counters = self
                .counters
                .lock()
                .map_err(|e| ServiceError(format!("Lock error: {e}")))?;
            let old_value = *counters.get(name).unwrap_or(&0);
            counters.insert(name.to_string(), 0);
            Ok(old_value)
        }
    }

    pub fn create_counter_registry() -> StatefulMethodRegistry<CounterService> {
        StatefulMethodRegistry::new()
            .register_fn("increment", |ctx: &CounterService, params, id| {
                let name = if let Some(ref params) = params {
                    params
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("default")
                } else {
                    "default"
                };

                match ctx.increment(name) {
                    Ok(value) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "counter": name,
                            "value": value
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
            .register_fn("get", |ctx: &CounterService, params, id| {
                let name = if let Some(ref params) = params {
                    params
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("default")
                } else {
                    "default"
                };

                match ctx.get(name) {
                    Ok(value) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "counter": name,
                            "value": value
                        }))
                        .id(id)
                        .build()),
                    Err(e) => Ok(ResponseBuilder::new()
                        .error(ErrorBuilder::new(-32001, e.to_string()).build())
                        .id(id)
                        .build()),
                }
            })
            .register_fn("reset", |ctx: &CounterService, params, id| {
                let name = if let Some(ref params) = params {
                    params
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("default")
                } else {
                    "default"
                };

                match ctx.reset(name) {
                    Ok(old_value) => Ok(ResponseBuilder::new()
                        .success(serde_json::json!({
                            "counter": name,
                            "old_value": old_value,
                            "new_value": 0
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

    pub fn run_server() -> Result<(), Box<dyn std::error::Error>> {
        let counter_service = CounterService::new(1000);
        let registry = create_counter_registry();

        let processor = StatefulProcessor::builder(counter_service)
            .registry(registry)
            .build()?;

        let server = TcpServer::builder("127.0.0.1:3040")
            .processor(processor)
            .build()?;

        println!("Stateful Counter TCP server listening on 127.0.0.1:3040");
        println!("Available methods: increment, get, reset");
        println!(
            "Example: {{\"jsonrpc\":\"2.0\",\"method\":\"increment\",\"params\":{{\"name\":\"user_clicks\"}},\"id\":1}}"
        );

        Ok(server.run()?)
    }
}

#[cfg(feature = "tcp")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    example::run_server()
}

#[cfg(not(feature = "tcp"))]
fn main() {
    println!("This example requires the 'tcp' feature to be enabled.");
    println!("Run with: cargo run --example tcp_stateful_server --features tcp");
}
