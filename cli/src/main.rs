//! # ash-rpc-gen CLI Tool
//! 
//! Generate ready-to-use JSON-RPC implementation files from method specifications.

use clap::Parser;
use std::fs;
use std::path::Path;

/// Command-line arguments for the ash-rpc-gen tool
#[derive(Parser)]
#[command(name = "ash-rpc-gen")]
#[command(about = "Generate ready-to-use JSON-RPC implementation files")]
#[command(version = "0.1.0")]
struct Args {
    /// Method name to generate implementation for
    #[arg(short, long)]
    method: String,
    
    /// Output file path for the generated Rust file
    #[arg(short, long)]
    output: String,
}

fn main() {
    let args = Args::parse();
    
    match generate_rpc_file(&args.method, &args.output) {
        Ok(()) => {
            println!("Successfully generated JSON-RPC implementation at: {}", args.output);
            println!("Method: {}", args.method);
        }
        Err(e) => {
            eprintln!("Error generating file: {}", e);
            std::process::exit(1);
        }
    }
}

/// Generate a complete JSON-RPC implementation file
fn generate_rpc_file(method_name: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent)?;
    }
    
    let content = generate_rpc_implementation(method_name);
    fs::write(output_path, content)?;
    
    Ok(())
}

/// Generate the implementation code for a JSON-RPC method
fn generate_rpc_implementation(method_name: &str) -> String {
    format!(r#"//! Generated JSON-RPC implementation for method: {}
//! 
//! This file contains a ready-to-use JSON-RPC server implementation.
//! Fill in the TODOs to complete your implementation.
//!
//! To use transport features, add them to your Cargo.toml:
//! ```toml
//! [dependencies]
//! ash-rpc-core = {{ features = ["tcp"] }}  # For TCP transport
//! # OR ash-rpc-core = {{ features = ["tcp-stream"] }}  # For TCP streaming
//! # OR ash-rpc-core = {{ features = ["axum"] }}  # For HTTP/Axum transport
//! ```

use ash_rpc_core::*;

fn main() {{
    // Create the JSON-RPC method registry
    let registry = MethodRegistry::new()
        .register("{}", |params, id| {{
            // TODO: Implement your method logic here
            // The params contain the input parameters for your method
            // Return appropriate success or error responses
            
            // TODO: Parse and validate parameters
            // Example: 
            // if let Some(params) = params {{
            //     if let Ok(input) = serde_json::from_value::<YourInputType>(params) {{
            //         // Process input...
            //     }} else {{
            //         return rpc_invalid_params!("Invalid parameters", id);
            //     }}
            // }} else {{
            //     return rpc_invalid_params!("Missing parameters", id);
            // }}
            
            // TODO: Implement your business logic here
            // Example:
            // let result = your_business_logic(input);
            
            // TODO: Return success response with your result
            // Example:
            // rpc_success!(result, id)
            
            // Placeholder response - replace with your implementation
            rpc_success!("Method {} executed successfully", id)
        }});

    // Example: Process a test request
    let request = rpc_request!("{}", ["param1", "param2"], 1);
    let message = Message::Request(request);
    
    if let Some(response) = registry.process_message(message) {{
        println!("Response: {{}}", serde_json::to_string_pretty(&response).unwrap());
    }}
    
    // TODO: Set up your transport layer
    // Remember to enable the appropriate feature in Cargo.toml!
    
    // For TCP server (requires "tcp" feature):
    // #[tokio::main]
    // async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    //     use ash_rpc_core::transport::tcp::TcpServer;
    //     let server = TcpServer::builder("127.0.0.1:8080")
    //         .processor(registry)
    //         .build()?;
    //     server.run()
    // }}
    
    // For TCP streaming server (requires "tcp-stream" feature):
    // #[tokio::main]
    // async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    //     use ash_rpc_core::transport::tcp_stream::TcpStreamServer;
    //     let server = TcpStreamServer::builder("127.0.0.1:8080")
    //         .processor(registry)
    //         .build();
    //     server.run().await
    // }}
    
    // For HTTP server with Axum (requires "axum" feature):
    // #[tokio::main] 
    // async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    //     use ash_rpc_core::transport::axum::{{AxumRpcLayer, AxumRpcLayerBuilder}};
    //     use axum::{{routing::post, Router}};
    //     
    //     let rpc_layer = AxumRpcLayerBuilder::new()
    //         .processor(registry)
    //         .build();
    //     
    //     let app = Router::new()
    //         .route("/rpc", post(|| async {{ "RPC endpoint" }}))
    //         .layer(rpc_layer);
    //     
    //     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    //     axum::serve(listener, app).await?;
    //     Ok(())
    // }}
}}

// TODO: Define your input/output types
// Example:
// #[derive(serde::Deserialize)]
// struct YourInputType {{
//     // Define your input parameters
// }}
//
// #[derive(serde::Serialize)]
// struct YourOutputType {{
//     // Define your output structure  
// }}

// TODO: Implement your business logic functions
// Example:
// fn your_business_logic(input: YourInputType) -> YourOutputType {{
//     // Implement your logic here
//     todo!("Implement your business logic")
// }}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_{}_method() {{
        let registry = MethodRegistry::new()
            .register("{}", |params, id| {{
                // TODO: Add your test implementation
                rpc_success!("test result", id)
            }});

        let request = rpc_request!("{}", serde_json::json!(["test"]), 1);
        let message = Message::Request(request);
        
        if let Some(response) = registry.process_message(message) {{
            assert!(response.is_success());
            // TODO: Add more specific assertions for your method
        }}
    }}
}}
"#, method_name, method_name, method_name, method_name, method_name, method_name, method_name)
}
