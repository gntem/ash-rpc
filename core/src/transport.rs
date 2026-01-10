/// TCP transport implementation for JSON-RPC servers.
///
/// This module provides TCP-based transport layers for JSON-RPC communication,
/// including both simple TCP servers and streaming TCP servers that can handle
/// multiple requests per connection.
///
/// # Features
/// - Simple TCP server for one-request-per-connection
/// - Streaming TCP server for persistent connections
/// - Line-delimited JSON message framing
/// - Thread-based concurrent request handling
///
/// # Example
/// ```rust,no_run
/// use ash_rpc_core::{MethodRegistry, transport::tcp::TcpServerBuilder};
///
/// let mut registry = MethodRegistry::new();
/// // ... register methods ...
///
/// let server = TcpServerBuilder::new("127.0.0.1:8080")
///     .processor(registry)
///     .build()
///     .expect("Failed to create server");
///     
/// server.run().expect("Server failed");
/// ```
#[cfg(feature = "tcp")]
pub mod tcp {
    use crate::{Message, MessageProcessor};
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::runtime::Runtime;

    /// Builder for creating TCP JSON-RPC servers.
    ///
    /// Provides a fluent API for configuring and building TCP servers
    /// that can handle JSON-RPC requests over TCP connections.
    pub struct TcpServerBuilder {
        addr: String,
        processor: Option<Arc<dyn MessageProcessor + Send + Sync>>,
    }

    impl TcpServerBuilder {
        pub fn new(addr: impl Into<String>) -> Self {
            Self {
                addr: addr.into(),
                processor: None,
            }
        }

        pub fn processor<P>(mut self, processor: P) -> Self
        where
            P: MessageProcessor + Send + Sync + 'static,
        {
            self.processor = Some(Arc::new(processor));
            self
        }

        pub fn build(self) -> Result<TcpServer, std::io::Error> {
            let processor = self.processor.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Processor not set")
            })?;

            Ok(TcpServer {
                addr: self.addr,
                processor,
            })
        }
    }

    pub struct TcpServer {
        addr: String,
        processor: Arc<dyn MessageProcessor + Send + Sync>,
    }

    impl TcpServer {
        pub fn builder(addr: impl Into<String>) -> TcpServerBuilder {
            TcpServerBuilder::new(addr)
        }

        pub fn run(&self) -> Result<(), std::io::Error> {
            let rt = Runtime::new()?;
            rt.block_on(self.run_async())
        }

        async fn run_async(&self) -> Result<(), std::io::Error> {
            let listener = TcpListener::bind(&self.addr).await?;
            println!("TCP RPC Server listening on {}", self.addr);

            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let processor = Arc::clone(&self.processor);
                        tokio::spawn(async move {
                            if let Err(e) = handle_client(stream, processor).await {
                                eprintln!("Error handling client: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Error accepting connection: {e}");
                    }
                }
            }
        }
    }

    async fn handle_client(
        stream: TcpStream,
        processor: Arc<dyn MessageProcessor + Send + Sync>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                break;
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<Message>(line) {
                Ok(message) => {
                    let response_opt = processor.process_message(message).await;
                    if let Some(response) = response_opt {
                        let response_json = serde_json::to_string(&response)?;
                        writer.write_all(response_json.as_bytes()).await?;
                        writer.write_all(b"\n").await?;
                        writer.flush().await?;
                    }
                }
                Err(e) => {
                    let error_response = crate::ResponseBuilder::new()
                        .error(
                            crate::ErrorBuilder::new(
                                crate::error_codes::PARSE_ERROR,
                                format!("Parse error: {e}"),
                            )
                            .build(),
                        )
                        .id(None)
                        .build();

                    let error_json = serde_json::to_string(&error_response)?;
                    writer.write_all(error_json.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(feature = "tcp-stream")]
pub mod tcp_stream {
    use crate::{Message, MessageProcessor};
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::mpsc;

    pub struct TcpStreamServerBuilder {
        addr: String,
        processor: Option<Arc<dyn MessageProcessor + Send + Sync>>,
    }

    impl TcpStreamServerBuilder {
        pub fn new(addr: impl Into<String>) -> Self {
            Self {
                addr: addr.into(),
                processor: None,
            }
        }

        pub fn processor<P>(mut self, processor: P) -> Self
        where
            P: MessageProcessor + Send + Sync + 'static,
        {
            self.processor = Some(Arc::new(processor));
            self
        }

        pub fn build(self) -> Result<TcpStreamServer, std::io::Error> {
            let processor = self.processor.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Processor not set")
            })?;

            Ok(TcpStreamServer {
                addr: self.addr,
                processor,
            })
        }
    }

    pub struct TcpStreamServer {
        addr: String,
        processor: Arc<dyn MessageProcessor + Send + Sync>,
    }

    impl TcpStreamServer {
        pub fn builder(addr: impl Into<String>) -> TcpStreamServerBuilder {
            TcpStreamServerBuilder::new(addr)
        }

        pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
            let listener = TcpListener::bind(&self.addr).await?;
            println!("TCP Stream RPC Server listening on {}", self.addr);

            loop {
                let (stream, addr) = listener.accept().await?;
                println!("New connection from: {addr}");

                let processor = Arc::clone(&self.processor);
                tokio::spawn(async move {
                    if let Err(e) = handle_stream_client(stream, processor).await {
                        eprintln!("Error handling client {addr}: {e}");
                    }
                });
            }
        }
    }

    async fn handle_stream_client(
        stream: TcpStream,
        processor: Arc<dyn MessageProcessor + Send + Sync>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (reader, writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let (tx, mut rx) = mpsc::channel::<String>(100);

        tokio::spawn(async move {
            let mut writer = writer;
            while let Some(response) = rx.recv().await {
                if writer.write_all(response.as_bytes()).await.is_err()
                    || writer.write_all(b"\n").await.is_err()
                    || writer.flush().await.is_err()
                {
                    break;
                }
            }
        });

        let mut line = String::new();
        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                break;
            }

            let line_content = line.trim();
            if line_content.is_empty() {
                continue;
            }

            match serde_json::from_str::<Message>(line_content) {
                Ok(message) => {
                    if let Some(response) = processor.process_message(message).await {
                        let response_json = serde_json::to_string(&response)?;
                        if tx.send(response_json).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let error_response = crate::ResponseBuilder::new()
                        .error(
                            crate::ErrorBuilder::new(
                                crate::error_codes::PARSE_ERROR,
                                format!("Parse error: {e}"),
                            )
                            .build(),
                        )
                        .id(None)
                        .build();

                    let response_json = serde_json::to_string(&error_response)?;
                    if tx.send(response_json).await.is_err() {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    pub struct TcpStreamClientBuilder {
        addr: String,
    }

    impl TcpStreamClientBuilder {
        pub fn new(addr: impl Into<String>) -> Self {
            Self { addr: addr.into() }
        }

        pub async fn connect(self) -> Result<TcpStreamClient, Box<dyn std::error::Error>> {
            let stream = TcpStream::connect(&self.addr).await?;
            Ok(TcpStreamClient::new(stream))
        }
    }

    pub struct TcpStreamClient {
        tx: mpsc::Sender<String>,
        rx: mpsc::Receiver<String>,
    }

    impl TcpStreamClient {
        fn new(stream: TcpStream) -> Self {
            let (reader, writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            let (write_tx, mut write_rx) = mpsc::channel::<String>(100);
            let (read_tx, read_rx) = mpsc::channel::<String>(100);

            tokio::spawn(async move {
                let mut writer = writer;
                while let Some(message) = write_rx.recv().await {
                    if writer.write_all(message.as_bytes()).await.is_err() {
                        break;
                    }
                    if writer.write_all(b"\n").await.is_err() {
                        break;
                    }
                    if writer.flush().await.is_err() {
                        break;
                    }
                }
            });

            tokio::spawn(async move {
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => break,
                        Ok(_) => {
                            let line_content = line.trim();
                            if !line_content.is_empty()
                                && read_tx.send(line_content.to_string()).await.is_err()
                            {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });

            Self {
                tx: write_tx,
                rx: read_rx,
            }
        }

        pub async fn send_message(
            &self,
            message: &Message,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let json = serde_json::to_string(message)?;
            self.tx.send(json).await.map_err(|e| e.into())
        }

        pub async fn recv_message(
            &mut self,
        ) -> Result<Option<Message>, Box<dyn std::error::Error>> {
            if let Some(response) = self.rx.recv().await {
                let message: Message = serde_json::from_str(&response)?;
                Ok(Some(message))
            } else {
                Ok(None)
            }
        }
    }
}

/// TLS-enabled TCP streaming transport implementation.
///
/// Provides secure TCP streaming with TLS encryption using rustls.
/// Supports server-side TLS with configurable certificates and keys.
///
/// # Example
/// ```rust,no_run
/// use ash_rpc_core::transport::tcp_stream_tls::TlsConfig;
/// use ash_rpc_core::{MethodRegistry, transport::tcp_stream_tls::TcpStreamTlsServer};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let registry = MethodRegistry::empty();
/// 
/// let tls_config = TlsConfig::from_pem_files(
///     "certs/cert.pem",
///     "certs/key.pem"
/// )?;
///
/// let server = TcpStreamTlsServer::builder("127.0.0.1:8443")
///     .processor(registry)
///     .tls_config(tls_config)
///     .build()?;
///     
/// server.run().await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "tcp-stream-tls")]
pub mod tcp_stream_tls {
    use crate::{Message, MessageProcessor};
    use rustls_pemfile::{certs, pkcs8_private_keys};
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
    use tokio::net::{TcpListener, TcpStream};
    use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use tokio_rustls::rustls::ServerConfig;
    use tokio_rustls::TlsAcceptor;

    /// TLS configuration for secure connections
    #[derive(Clone)]
    pub struct TlsConfig {
        acceptor: TlsAcceptor,
    }

    impl TlsConfig {
        /// Create TLS config from PEM files
        pub fn from_pem_files(
            cert_path: impl AsRef<Path>,
            key_path: impl AsRef<Path>,
        ) -> Result<Self, Box<dyn std::error::Error>> {
            let cert_file = File::open(cert_path)?;
            let key_file = File::open(key_path)?;

            let cert_reader = &mut BufReader::new(cert_file);
            let key_reader = &mut BufReader::new(key_file);

            let certs: Vec<CertificateDer> = certs(cert_reader)
                .collect::<Result<Vec<_>, _>>()?;

            let mut keys: Vec<PrivateKeyDer> = pkcs8_private_keys(key_reader)
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(PrivateKeyDer::from)
                .collect();

            if keys.is_empty() {
                return Err("No private keys found in key file".into());
            }

            let config = ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(certs, keys.remove(0))?;

            Ok(Self {
                acceptor: TlsAcceptor::from(Arc::new(config)),
            })
        }

        /// Create TLS config from PEM bytes
        pub fn from_pem_bytes(
            cert_pem: &[u8],
            key_pem: &[u8],
        ) -> Result<Self, Box<dyn std::error::Error>> {
            let cert_reader = &mut BufReader::new(cert_pem);
            let key_reader = &mut BufReader::new(key_pem);

            let certs: Vec<CertificateDer> = certs(cert_reader)
                .collect::<Result<Vec<_>, _>>()?;

            let mut keys: Vec<PrivateKeyDer> = pkcs8_private_keys(key_reader)
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(PrivateKeyDer::from)
                .collect();

            if keys.is_empty() {
                return Err("No private keys found in key data".into());
            }

            let config = ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(certs, keys.remove(0))?;

            Ok(Self {
                acceptor: TlsAcceptor::from(Arc::new(config)),
            })
        }
    }

    pub struct TcpStreamTlsServerBuilder {
        addr: String,
        processor: Option<Arc<dyn MessageProcessor + Send + Sync>>,
        tls_config: Option<TlsConfig>,
    }

    impl TcpStreamTlsServerBuilder {
        pub fn new(addr: impl Into<String>) -> Self {
            Self {
                addr: addr.into(),
                processor: None,
                tls_config: None,
            }
        }

        pub fn processor<P>(mut self, processor: P) -> Self
        where
            P: MessageProcessor + Send + Sync + 'static,
        {
            self.processor = Some(Arc::new(processor));
            self
        }

        pub fn tls_config(mut self, config: TlsConfig) -> Self {
            self.tls_config = Some(config);
            self
        }

        pub fn build(self) -> Result<TcpStreamTlsServer, std::io::Error> {
            let processor = self.processor.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Processor not set")
            })?;

            let tls_config = self.tls_config.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "TLS config not set")
            })?;

            Ok(TcpStreamTlsServer {
                addr: self.addr,
                processor,
                tls_config,
            })
        }
    }

    pub struct TcpStreamTlsServer {
        addr: String,
        processor: Arc<dyn MessageProcessor + Send + Sync>,
        tls_config: TlsConfig,
    }

    impl TcpStreamTlsServer {
        pub fn builder(addr: impl Into<String>) -> TcpStreamTlsServerBuilder {
            TcpStreamTlsServerBuilder::new(addr)
        }

        pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
            let listener = TcpListener::bind(&self.addr).await?;
            println!("TLS TCP Stream RPC Server listening on {}", self.addr);

            loop {
                let (stream, addr) = listener.accept().await?;
                println!("New TLS connection from: {addr}");

                let processor = Arc::clone(&self.processor);
                let acceptor = self.tls_config.acceptor.clone();

                tokio::spawn(async move {
                    match acceptor.accept(stream).await {
                        Ok(tls_stream) => {
                            if let Err(e) = handle_tls_client(tls_stream, processor).await {
                                eprintln!("Error handling TLS client {addr}: {e}");
                            }
                        }
                        Err(e) => {
                            eprintln!("TLS handshake failed for {addr}: {e}");
                        }
                    }
                });
            }
        }
    }

    async fn handle_tls_client<S>(
        stream: S,
        processor: Arc<dyn MessageProcessor + Send + Sync>,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let (reader, writer) = tokio::io::split(stream);
        let mut reader = TokioBufReader::new(reader);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);

        // Writer task
        tokio::spawn(async move {
            let mut writer = writer;
            while let Some(response) = rx.recv().await {
                if writer.write_all(response.as_bytes()).await.is_err()
                    || writer.write_all(b"\n").await.is_err()
                    || writer.flush().await.is_err()
                {
                    break;
                }
            }
        });

        // Reader/processor loop
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    let message_result: Result<Message, _> = serde_json::from_str(line.trim());

                    match message_result {
                        Ok(message) => {
                            if let Some(response) = processor.process_message(message).await {
                                if let Ok(response_json) = serde_json::to_string(&response) {
                                    if tx.send(response_json).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let error_response = crate::ResponseBuilder::new()
                                .error(
                                    crate::ErrorBuilder::new(
                                        crate::error_codes::PARSE_ERROR,
                                        format!("Parse error: {e}"),
                                    )
                                    .build(),
                                )
                                .id(None)
                                .build();

                            if let Ok(error_json) = serde_json::to_string(&error_response) {
                                if tx.send(error_json).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }

        Ok(())
    }

    /// TLS-enabled streaming client
    pub struct TcpStreamTlsClient {
        stream: tokio_rustls::client::TlsStream<TcpStream>,
    }

    impl TcpStreamTlsClient {
        /// Connect to a TLS server (for testing - accepts self-signed certs)
        pub async fn connect_insecure(
            addr: impl AsRef<str>,
        ) -> Result<Self, Box<dyn std::error::Error>> {
            use tokio_rustls::rustls::ClientConfig;
            use tokio_rustls::TlsConnector;

            // Create a client config that doesn't verify certificates (for testing only)
            let config = ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(NoVerifier))
                .with_no_client_auth();

            let connector = TlsConnector::from(Arc::new(config));
            let stream = TcpStream::connect(addr.as_ref()).await?;
            
            let domain = tokio_rustls::rustls::pki_types::ServerName::try_from("localhost")?;
            let tls_stream = connector.connect(domain.to_owned(), stream).await?;

            Ok(Self { stream: tls_stream })
        }

        /// Send a JSON-RPC request
        pub async fn send_request(
            &mut self,
            request: &crate::Request,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let json = serde_json::to_string(request)?;
            self.stream.write_all(json.as_bytes()).await?;
            self.stream.write_all(b"\n").await?;
            self.stream.flush().await?;
            Ok(())
        }

        /// Receive a JSON-RPC response
        pub async fn recv_response(
            &mut self,
        ) -> Result<crate::Response, Box<dyn std::error::Error>> {
            let mut reader = TokioBufReader::new(&mut self.stream);
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            let response: crate::Response = serde_json::from_str(line.trim())?;
            Ok(response)
        }
    }

    // Insecure certificate verifier for testing
    #[derive(Debug)]
    struct NoVerifier;

    impl tokio_rustls::rustls::client::danger::ServerCertVerifier for NoVerifier {
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _server_name: &tokio_rustls::rustls::pki_types::ServerName<'_>,
            _ocsp_response: &[u8],
            _now: tokio_rustls::rustls::pki_types::UnixTime,
        ) -> Result<tokio_rustls::rustls::client::danger::ServerCertVerified, tokio_rustls::rustls::Error> {
            Ok(tokio_rustls::rustls::client::danger::ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &tokio_rustls::rustls::DigitallySignedStruct,
        ) -> Result<tokio_rustls::rustls::client::danger::HandshakeSignatureValid, tokio_rustls::rustls::Error> {
            Ok(tokio_rustls::rustls::client::danger::HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &tokio_rustls::rustls::DigitallySignedStruct,
        ) -> Result<tokio_rustls::rustls::client::danger::HandshakeSignatureValid, tokio_rustls::rustls::Error> {
            Ok(tokio_rustls::rustls::client::danger::HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<tokio_rustls::rustls::SignatureScheme> {
            use tokio_rustls::rustls::SignatureScheme;
            vec![
                SignatureScheme::RSA_PKCS1_SHA256,
                SignatureScheme::RSA_PKCS1_SHA384,
                SignatureScheme::RSA_PKCS1_SHA512,
                SignatureScheme::ECDSA_NISTP256_SHA256,
                SignatureScheme::ECDSA_NISTP384_SHA384,
                SignatureScheme::RSA_PSS_SHA256,
                SignatureScheme::RSA_PSS_SHA384,
                SignatureScheme::RSA_PSS_SHA512,
                SignatureScheme::ED25519,
            ]
        }
    }
}
