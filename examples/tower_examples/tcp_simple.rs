use ash_rpc_contrib::JsonRpcLayer;
use ash_rpc_core::{Error, Request, Response, error_codes};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tower::{Service, ServiceBuilder};

#[derive(Debug, Deserialize)]
struct AddParams {
    a: f64,
    b: f64,
}

#[derive(Debug, Serialize)]
struct AddResult {
    sum: f64,
}

#[derive(Clone)]
struct CalculatorService;

impl Service<Request> for CalculatorService {
    type Response = Response;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        Box::pin(async move {
            match req.method() {
                "add" => {
                    let params: AddParams = match req.params() {
                        Some(params) => serde_json::from_value(params.clone()).map_err(|_| {
                            Error::new(
                                error_codes::INVALID_PARAMS,
                                "Invalid parameters for add method",
                            )
                        })?,
                        None => {
                            return Err(Error::new(
                                error_codes::INVALID_PARAMS,
                                "Missing parameters for add method",
                            ));
                        }
                    };

                    let result = AddResult {
                        sum: params.a + params.b,
                    };

                    Ok(Response::success(
                        serde_json::to_value(result).unwrap(),
                        req.id.clone(),
                    ))
                }
                _ => Err(Error::new(
                    error_codes::METHOD_NOT_FOUND,
                    format!("Method '{}' not found", req.method()),
                )),
            }
        })
    }
}

async fn handle_connection<S>(mut stream: TcpStream, mut service: S)
where
    S: Service<Request, Response = Response, Error = Error> + Clone + Send + 'static,
    S::Future: Send,
{
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let response = match serde_json::from_str::<Request>(line) {
                    Ok(request) => {
                        println!("Received request: {:?}", request);
                        match service.call(request).await {
                            Ok(resp) => resp,
                            Err(err) => Response::error(err, None),
                        }
                    }
                    Err(_) => {
                        let error =
                            Error::new(error_codes::PARSE_ERROR, "Invalid JSON-RPC request");
                        Response::error(error, None)
                    }
                };

                let response_json = serde_json::to_string(&response).unwrap();
                println!("Sending response: {}", response_json);

                if let Err(e) = writer
                    .write_all(format!("{}\n", response_json).as_bytes())
                    .await
                {
                    eprintln!("Failed to write response: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("Failed to read from socket: {}", e);
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting Tower TCP JSON-RPC Calculator Server on 127.0.0.1:8080");
    println!(
        "Connect with telnet and send: {{\"jsonrpc\":\"2.0\",\"method\":\"add\",\"params\":{{\"a\":5,\"b\":3}},\"id\":1}}"
    );

    let service = ServiceBuilder::new()
        .layer(JsonRpcLayer::new().validate_version(true))
        .service(CalculatorService);

    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        let (stream, addr) = listener.accept().await?;
        println!("New connection from: {}", addr);

        let service = service.clone();
        tokio::task::spawn(async move {
            handle_connection(stream, service).await;
            println!("Connection from {} closed", addr);
        });
    }
}
