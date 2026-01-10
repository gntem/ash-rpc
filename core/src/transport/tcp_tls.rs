//! TLS-enabled TCP streaming transport implementation for JSON-RPC servers.
//!
//! Provides secure TCP streaming with TLS encryption using rustls.

use super::security::SecurityConfig;
use crate::{Message, MessageProcessor};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};

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
        let cert_bytes = std::fs::read(cert_path)?;
        let key_bytes = std::fs::read(key_path)?;

        let certs = CertificateDer::pem_slice_iter(&cert_bytes)
            .collect::<Result<Vec<_>, _>>()?;

        let mut keys = PrivateKeyDer::pem_slice_iter(&key_bytes)
            .collect::<Result<Vec<_>, _>>()?;

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
        let certs = CertificateDer::pem_slice_iter(cert_pem)
            .collect::<Result<Vec<_>, _>>()?;

        let mut keys = PrivateKeyDer::pem_slice_iter(key_pem)
            .collect::<Result<Vec<_>, _>>()?;

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
    security_config: SecurityConfig,
}

impl TcpStreamTlsServerBuilder {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            processor: None,
            tls_config: None,
            security_config: SecurityConfig::default(),
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

    pub fn security_config(mut self, config: SecurityConfig) -> Self {
        self.security_config = config;
        self
    }

    pub fn max_connections(mut self, max: usize) -> Self {
        self.security_config.max_connections = max;
        self
    }

    pub fn max_request_size(mut self, size: usize) -> Self {
        self.security_config.max_request_size = size;
        self
    }

    pub fn request_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.security_config.request_timeout = timeout;
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
            security_config: self.security_config,
            active_connections: Arc::new(AtomicUsize::new(0)),
        })
    }
}

pub struct TcpStreamTlsServer {
    addr: String,
    processor: Arc<dyn MessageProcessor + Send + Sync>,
    tls_config: TlsConfig,
    security_config: SecurityConfig,
    active_connections: Arc<AtomicUsize>,
}

impl TcpStreamTlsServer {
    pub fn builder(addr: impl Into<String>) -> TcpStreamTlsServerBuilder {
        TcpStreamTlsServerBuilder::new(addr)
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        tracing::info!(
            addr = %self.addr,
            protocol = "tls",
            max_connections = self.security_config.max_connections,
            max_request_size = self.security_config.max_request_size,
            "server listening"
        );

        loop {
            let (stream, addr) = listener.accept().await?;

            let current_connections = self.active_connections.load(Ordering::Relaxed);

            // Check connection limit
            if self.security_config.max_connections > 0
                && current_connections >= self.security_config.max_connections
            {
                tracing::warn!(
                    remote_addr = %addr,
                    active_connections = current_connections,
                    max_connections = self.security_config.max_connections,
                    "connection limit reached, rejecting connection"
                );
                drop(stream);
                continue;
            }

            self.active_connections.fetch_add(1, Ordering::Relaxed);
            tracing::debug!(remote_addr = %addr, protocol = "tls", active_connections = current_connections + 1, "new connection");

            let processor = Arc::clone(&self.processor);
            let acceptor = self.tls_config.acceptor.clone();
            let security_config = self.security_config.clone();
            let active_connections = Arc::clone(&self.active_connections);

            tokio::spawn(async move {
                let result = match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        handle_tls_client(tls_stream, processor, security_config).await
                    }
                    Err(e) => {
                        tracing::warn!(remote_addr = %addr, error = %e, "tls handshake failed");
                        Err(e.into())
                    }
                };

                active_connections.fetch_sub(1, Ordering::Relaxed);

                if let Err(e) = result {
                    tracing::error!(remote_addr = %addr, error = %e, "tls client handler failed");
                }
            });
        }
    }
}

async fn handle_tls_client<S>(
    stream: S,
    processor: Arc<dyn MessageProcessor + Send + Sync>,
    security_config: SecurityConfig,
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

        // Apply idle timeout
        let read_result =
            match timeout(security_config.idle_timeout, reader.read_line(&mut line)).await {
                Ok(result) => result,
                Err(_) => {
                    tracing::debug!("connection idle timeout");
                    break;
                }
            };

        match read_result {
            Ok(0) => break,
            Ok(_) => {
                // Check max request size
                if security_config.max_request_size > 0
                    && line.len() > security_config.max_request_size
                {
                    tracing::warn!(
                        request_size = line.len(),
                        max_size = security_config.max_request_size,
                        "request size limit exceeded"
                    );
                    let error_response = crate::Response::error(
                        crate::ErrorBuilder::new(
                            crate::error_codes::INVALID_REQUEST,
                            "Request size limit exceeded".to_string(),
                        )
                        .build(),
                        None,
                    );
                    if let Ok(json) = serde_json::to_string(&error_response) {
                        let _ = tx.send(json).await;
                    }
                    break;
                }

                let message_result: Result<Message, _> = serde_json::from_str(line.trim());

                match message_result {
                    Ok(message) => {
                        if let Some(response) = processor.process_message(message).await
                            && let Ok(response_json) = serde_json::to_string(&response)
                            && tx.send(response_json).await.is_err()
                        {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "json-rpc parse failed");
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

                        if let Ok(error_json) = serde_json::to_string(&error_response)
                            && tx.send(error_json).await.is_err()
                        {
                            break;
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
        use tokio_rustls::TlsConnector;
        use tokio_rustls::rustls::ClientConfig;

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
    pub async fn recv_response(&mut self) -> Result<crate::Response, Box<dyn std::error::Error>> {
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
    ) -> Result<tokio_rustls::rustls::client::danger::ServerCertVerified, tokio_rustls::rustls::Error>
    {
        Ok(tokio_rustls::rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        tokio_rustls::rustls::Error,
    > {
        Ok(tokio_rustls::rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        tokio_rustls::rustls::Error,
    > {
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
