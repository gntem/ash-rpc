#!/bin/bash
# Generate self-signed TLS certificates for testing

set -e

CERT_DIR="certs"
CERT_FILE="$CERT_DIR/cert.pem"
KEY_FILE="$CERT_DIR/key.pem"

echo "=== Generating TLS Test Certificates ==="
echo ""

# Create directory if it doesn't exist
mkdir -p "$CERT_DIR"

# Generate self-signed certificate
echo "Generating self-signed certificate..."
openssl req -x509 -newkey rsa:4096 \
    -keyout "$KEY_FILE" \
    -out "$CERT_FILE" \
    -days 365 \
    -nodes \
    -subj '/CN=localhost' \
    2>/dev/null

echo " Certificate generated: $CERT_FILE"
echo " Private key generated: $KEY_FILE"
echo ""
echo "Certificate details:"
openssl x509 -in "$CERT_FILE" -text -noout | grep -E "Subject:|Issuer:|Not Before|Not After"
echo ""
echo " Done! You can now run the TLS example with:"
echo "   cd ../.. && cargo run --example tls_server --features tcp-stream-tls"
