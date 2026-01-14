# Jaeger Tracing Integration

This observability example now includes **Jaeger** for distributed tracing visualization.

## What's New

### Services Added
- **Jaeger All-in-One** container running on:
  - UI: <http://localhost:16686>
  - OTLP gRPC: port 4317
  - OTLP HTTP: port 4318

### Code Changes
- Added OpenTelemetry OTLP exporter configured to send traces to Jaeger
- Tracer initialized with service name `ash-rpc-server`
- All RPC requests automatically traced via `ObservableProcessor`

### Grafana Integration
- Jaeger datasource auto-provisioned in Grafana
- You can explore traces directly from Grafana or use the Jaeger UI

## Using Jaeger

### 1. Access the Jaeger UI

Navigate to <http://localhost:16686> after running `docker-compose up --build`

### 2. Search for Traces

- **Service**: Select `ash-rpc-server`
- **Operation**: Choose specific RPC methods (e.g., `process_message`)
- **Tags**: Filter by custom tags
- **Lookback**: Adjust time range
- Click **Find Traces**

### 3. Analyze Trace Details

Each trace shows:
- **Timeline**: Visual representation of span durations
- **Span Details**: Operation name, duration, start time
- **Tags**: RPC method, request ID, parameters
- **Logs**: Events during span execution (errors, etc.)

### 4. Key Features

- **Latency Analysis**: Identify slow operations
- **Error Tracking**: Find failed requests with stack traces
- **Dependency Mapping**: Visualize service interactions
- **Comparative Analysis**: Compare traces side-by-side

## Trace Information Captured

For each RPC request, you'll see:
- RPC method name (e.g., `ping`, `add`, `multiply`)
- Request ID
- JSON-RPC version
- Duration and timing
- Success/failure status
- Error details (if any)

## Example Queries

### Find Slow Requests
- Min Duration: `500ms`
- Operation: `process_message`

### Find Failed Requests
- Tags: `error=true`

### Find Specific Method
- Tags: `rpc.method=add`

## Architecture

```text
RPC Request → ObservableProcessor
                ├─ PrometheusMetrics (counters, histograms)
                ├─ TracingProcessor (OpenTelemetry spans)
                └─ Logger (structured logs)

TracingProcessor → OTLP Exporter → Jaeger Collector → Jaeger Storage → Jaeger UI
```

## Tips

1. **Generate Traffic**: Run `./test-requests.sh` to generate diverse traces
2. **Compare Traces**: Use Jaeger's compare feature to analyze similar requests
3. **Export Traces**: Download traces as JSON for further analysis
4. **Deep Dive**: Click on individual spans to see detailed timing breakdowns

## Troubleshooting

### No traces appearing?

```bash
# Check if Jaeger is running
docker ps | grep jaeger

# Check logs
docker logs jaeger

# Verify OTLP endpoint is reachable from server
docker exec ash-rpc-server nc -zv jaeger 4317
```

### Traces not exported?

Check server logs for OpenTelemetry initialization:
```bash
docker logs ash-rpc-server | grep -i "telemetry"
```
