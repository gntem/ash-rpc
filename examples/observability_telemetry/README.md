# ASH-RPC Observability Telemetry Example

This example demonstrates a complete observability setup for an ASH-RPC server with Prometheus metrics, OpenTelemetry tracing, and Grafana visualization.

## Features

- **HTTP JSON-RPC Server** with multiple methods (ping, echo, add, multiply, slow_operation, always_fails)
- **Prometheus Metrics** tracking requests, latency, errors, and connections
- **OpenTelemetry Tracing** with distributed trace collection and visualization via Jaeger
- **Grafana Dashboard** with pre-configured visualizations
- **Structured Logging** using slog-rs outputting to stdout
- **Docker Compose** setup for easy deployment

## Quick Start

### Prerequisites

- Docker and Docker Compose
- curl (for testing)
- jq (optional, for pretty JSON output)

### 1. Build and Start Services

```bash
cd examples/observability_telemetry
docker-compose up --build
```

This will start:
- **ASH-RPC Server** on `http://localhost:3000`
- **Prometheus** on `http://localhost:9090`
- **Grafana** on `http://localhost:3001`
- **Jaeger UI** on `http://localhost:16686`

### 2. Run Test Requests

In a separate terminal:

```bash
# Make the script executable
chmod +x test-requests.sh

# Run 20 test requests
./test-requests.sh
```

```bash
# For higher load testing, you can run multiple instances
for i in {1..1000}; do ./test-requests.sh & done; wait
```

### 3. View Observability Data

#### Grafana Dashboard

- URL: <http://localhost:3001>
- Username: `admin`
- Password: `admin`
- Navigate to "ASH-RPC Observability Dashboard"

The dashboard shows:

- Request rate by method
- Total request rate gauge
- Request duration percentiles (p50, p95, p99)
- Error rate by method
- Active connections
- Total requests counter
- Total errors counter
- Success rate percentage

#### Jaeger Tracing UI

- URL: <http://localhost:16686>
- Service: `ash-rpc-server`
- View distributed traces for each RPC request
- Analyze request spans, timings, and dependencies

The Jaeger UI provides:

- **Search**: Find traces by service, operation, tags, and duration
- **Trace Timeline**: Visualize the complete request flow with span details
- **Service Dependencies**: See how services interact (useful in microservices)
- **Span Details**: Examine individual operation timings and metadata

#### Prometheus

- URL: <http://localhost:9090>
- Query examples:
  - `rate(ash_rpc_requests_total[1m])` - Request rate
  - `ash_rpc_request_duration_seconds_bucket` - Latency distribution
  - `ash_rpc_errors_total` - Error counts

#### Raw Metrics

- URL: <http://localhost:3000/metrics>
- View Prometheus-formatted metrics directly

### 4. View Logs

```bash
# View server logs
docker logs -f ash-rpc-server
```

## Available RPC Methods

| Method | Parameters | Description |
|--------|-----------|-------------|
| `ping` | none | Returns "pong" |
| `echo` | any | Echoes back the parameter |
| `add` | `[num1, num2]` | Adds two numbers |
| `multiply` | `[num1, num2]` | Multiplies two numbers |
| `slow_operation` | none | Simulates slow request (500ms) |
| `always_fails` | none | Always returns an error (for testing) |
| `get_metrics` | none | Returns Prometheus metrics via RPC |

## Manual Testing

```bash
# Ping
curl -X POST http://localhost:3000/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"ping","id":1}'

# Add numbers
curl -X POST http://localhost:3000/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"add","params":[10,20],"id":2}'

# Echo
curl -X POST http://localhost:3000/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"echo","params":"Hello World","id":3}'
```

## Architecture

```
┌─────────────────┐
│  Test Script    │
│  (20 requests)  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌──────────────┐
│  ASH-RPC Server │────▶│  Prometheus  │
│   (Port 3000)   │     │  (Port 9090) │
│                 │     └──────┬───────┘
│  - MethodReg    │            │
│  - Observable   │            │
│  - Prometheus   │            ▼
│  - Slog Logger  │     ┌──────────────┐
└─────────────────┘     │   Grafana    │
                        │  (Port 3001) │
                        │  Dashboard   │
                        └──────────────┘
```

## Metrics Collected

- `ash_rpc_requests_total{method}` - Counter of total requests per method
- `ash_rpc_request_duration_seconds{method}` - Histogram of request durations
- `ash_rpc_errors_total{method}` - Counter of errors per method
- `ash_rpc_active_connections` - Gauge of current active connections

## Cleanup

```bash
# Stop services
docker-compose down

# Remove volumes (optional)
docker-compose down -v
```

## Troubleshooting

### Server not starting
```bash
# Check logs
docker logs ash-rpc-server

# Rebuild without cache
docker-compose build --no-cache
docker-compose up
```

### Grafana dashboard not showing data
1. Wait 10-15 seconds for Prometheus to scrape metrics
2. Run the test script to generate requests
3. Check Prometheus is scraping: http://localhost:9090/targets
4. Verify metrics exist: http://localhost:3000/metrics

### Permission issues with test script
```bash
chmod +x test-requests.sh
```

## Development

To modify the server code:
1. Edit `src/main.rs`
2. Rebuild: `docker-compose up --build`

To modify the dashboard:
1. Edit `grafana/dashboards/ash-rpc-dashboard.json`
2. Restart Grafana: `docker-compose restart grafana`
