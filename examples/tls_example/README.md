# TLS-Enabled JSON-RPC Server Example

This example demonstrates how to use ash-rpc-core with TLS encryption for secure communication.

## Prerequisites

- OpenSSL installed on your system
- `tcp-stream-tls` feature enabled

## Quick Start

### 1. Generate Test Certificates

From this directory, run:

```bash
./generate_certs.sh
```

This will create self-signed certificates in the `certs/` directory:
- `certs/cert.pem` - Server certificate
- `certs/key.pem` - Private key

**Note:** These are self-signed certificates for testing only. For production, use certificates from a trusted Certificate Authority.

### 2. Run the Example

From the repository root:

```bash
cargo run --example tls_server --features tcp-stream-tls
```

Or from this directory:

```bash
cd ../.. && cargo run --example tls_server --features tcp-stream-tls
```

## What This Example Demonstrates

- **TLS Configuration**: Loading certificates and keys from PEM files
- **Secure Server**: Running a TLS-enabled TCP streaming server
- **Encrypted Communication**: All JSON-RPC traffic is encrypted
- **Client Connection**: Testing with a TLS client (insecure mode for self-signed certs)

## Features Used

- `tcp-stream-tls` - Enables TLS support with tokio-rustls

## Security Notes

⚠️ **Important**: This example uses self-signed certificates and an insecure client verifier for testing purposes only.

For production use:

- Use certificates from a trusted CA
- Enable proper certificate verification on clients
- Configure appropriate cipher suites and TLS versions
- Implement certificate rotation and renewal

## Code Structure

```rust
// Create TLS configuration
let tls_config = TlsConfig::from_pem_files(
    "examples/tls_example/certs/cert.pem",
    "examples/tls_example/certs/key.pem"
)?;

// Build TLS-enabled server
let server = TcpStreamTlsServer::builder("127.0.0.1:8443")
    .processor(registry)
    .tls_config(tls_config)
    .build()?;

// Run server
server.run().await?;
```

## Client Connection

```rust
// Connect (insecure for self-signed certs)
let mut client = TcpStreamTlsClient::connect_insecure("127.0.0.1:8443").await?;

// Send request
let request = rpc_request!("ping", 1);
client.send_request(&request).await?;

// Receive response
let response = client.recv_response().await?;
```
