# Quick Start Guide

## Start the Demo

```bash
cd examples/observability_telemetry
docker-compose up --build
```

Wait for all services to start (about 1-2 minutes).

## Run Tests

**Linux/Mac:**
```bash
chmod +x test-requests.sh
./test-requests.sh
```

## Access Dashboards

- **Grafana**: http://localhost:3001 (admin/admin)
- **Prometheus**: http://localhost:9090
- **Metrics API**: http://localhost:3000/metrics
- **Health Check**: http://localhost:3000/health

## View Logs

```bash
docker logs -f ash-rpc-server
```

## Clean Up

```bash
docker-compose down -v
```

## What You'll See

The Grafana dashboard displays:
- Real-time request rates per method
- Latency percentiles (p50, p95, p99)
- Error rates and counts
- Active connections
- Success rate percentage

The test script generates diverse traffic patterns:
- 5 ping requests (fast, successful)
- 5 echo requests (varied parameters)
- 4 math operations (add/multiply)
- 2 slow operations (500ms each, shows latency impact)
- 2 failing requests (shows error metrics)
- 1 invalid request (parameter validation error)

This creates clear visualization of:
- ✅ Request volume and distribution
- ⏱️ Performance characteristics
- ❌ Error handling
- 📊 Real-time metrics updates
