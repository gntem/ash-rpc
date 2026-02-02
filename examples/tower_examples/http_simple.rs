use ash_rpc::JsonRpcLayer;
use ash_rpc::{Error, Request, Response, error_codes};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::TcpListener;
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

async fn handle_json_rpc<S>(request_body: String, mut service: S) -> String
where
    S: Service<Request, Response = Response, Error = Error>,
    S::Future: Send,
{
    let json_rpc_req: Request = match serde_json::from_str(&request_body) {
        Ok(req) => req,
        Err(_) => {
            let error = Error::new(error_codes::PARSE_ERROR, "Invalid JSON-RPC request");
            let response = Response::error(error, None);
            return serde_json::to_string(&response).unwrap();
        }
    };

    let response = match service.call(json_rpc_req).await {
        Ok(resp) => resp,
        Err(err) => Response::error(err, None),
    };

    serde_json::to_string(&response).unwrap()
}

async fn handle_http_connection<S>(mut stream: tokio::net::TcpStream, service: S)
where
    S: Service<Request, Response = Response, Error = Error> + Clone + Send + 'static,
    S::Future: Send,
{
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut headers = Vec::new();
    let mut line = String::new();

    loop {
        line.clear();
        if let Ok(0) = reader.read_line(&mut line).await {
            return; // EOF
        }

        if line.trim().is_empty() {
            break; // End of headers
        }
        headers.push(line.clone());
    }

    let content_length = headers
        .iter()
        .find(|h| h.to_lowercase().starts_with("content-length:"))
        .and_then(|h| h.split(':').nth(1))
        .and_then(|len| len.trim().parse::<usize>().ok())
        .unwrap_or(0);

    if content_length == 0 {
        let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
        let _ = writer.write_all(response.as_bytes()).await;
        return;
    }

    let mut body = vec![0u8; content_length];
    if (tokio::io::AsyncReadExt::read_exact(&mut reader, &mut body).await).is_err() {
        let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
        let _ = writer.write_all(response.as_bytes()).await;
        return;
    }

    let body_str = String::from_utf8_lossy(&body);
    let json_response = handle_json_rpc(body_str.to_string(), service).await;

    let http_response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}",
        json_response.len(),
        json_response
    );

    let _ = writer.write_all(http_response.as_bytes()).await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting Tower HTTP JSON-RPC Calculator Server on http://127.0.0.1:3000");
    println!(
        "Try: curl -X POST http://127.0.0.1:3000 -H \"Content-Type: application/json\" -d @test.json"
    );
    println!(
        "Or create test.json with: {{\"jsonrpc\":\"2.0\",\"method\":\"add\",\"params\":{{\"a\":5,\"b\":3}},\"id\":1}}"
    );

    let service = ServiceBuilder::new()
        .layer(JsonRpcLayer::new().validate_version(true))
        .service(CalculatorService);

    let listener = TcpListener::bind("127.0.0.1:3000").await?;

    loop {
        let (stream, addr) = listener.accept().await?;
        println!("New HTTP connection from: {}", addr);

        let service = service.clone();
        tokio::task::spawn(async move {
            handle_http_connection(stream, service).await;
            println!("HTTP connection from {} closed", addr);
        });
    }
}
