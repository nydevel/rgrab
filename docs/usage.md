# Using rgrab

## Starting the Server

```bash
# Minimal startup (defaults: port 3000, DB at ./data/rgrab)
rgrab

# With CLI parameters
rgrab --data-dir /var/lib/rgrab --listen 0.0.0.0:3000 --log-level info

# With TOML config
rgrab --config /etc/rgrab/rgrab.toml

# Dev mode via cargo
cargo run -p server -- --config rgrab.toml --log-level debug
```

## TUI Client

The TUI client connects to a running server via HTTP.

```bash
# Connect to local server
rgrab-tui

# Connect to a remote server
rgrab-tui --server http://192.168.1.100:3000

# Dev mode via cargo
cargo run -p tui -- --server http://localhost:3000
```

### TUI Controls

| Key         | Action                            |
|-------------|-----------------------------------|
| `Tab`       | Switch between Logs / Traces      |
| `j/k`, `Up/Down` | Scroll                      |
| `h/l`, `Left/Right` | Switch sidebar / main panel |
| `/`         | Enter search mode                 |
| `Enter`     | Select label / expand trace       |
| `Esc`       | Exit search / close spans         |
| `L`         | Toggle live tail                  |
| `r`         | Refresh data                      |
| `+/-`       | Increase/decrease limit           |
| `s`         | Toggle sort order                 |
| `1-6`       | Toggle log levels (TRACE..FATAL)  |
| `q`         | Quit                              |

When the server is unavailable, the TUI shows a popup with the server address and error. Reconnection happens automatically when live tail is enabled.

---

## Connecting Applications to rgrab

### Sending Logs (JSON)

```bash
curl -X POST http://localhost:3000/v1/logs \
  -H 'Content-Type: application/json' \
  -d '[
    {
      "timestamp": "2025-01-15T10:30:00Z",
      "level": "INFO",
      "message": "User logged in",
      "labels": {
        "service": "auth",
        "environment": "production",
        "host": "web-01"
      },
      "trace_id": "abc123",
      "span_id": "span456"
    },
    {
      "timestamp": "2025-01-15T10:30:01Z",
      "level": "ERROR",
      "message": "Database connection failed",
      "labels": {
        "service": "api",
        "environment": "production"
      },
      "trace_id": null,
      "span_id": null
    }
  ]'
```

Log levels: `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`, `FATAL`.

### Sending Traces (JSON)

```bash
curl -X POST http://localhost:3000/v1/traces \
  -H 'Content-Type: application/json' \
  -d '[
    {
      "trace_id": "abc123",
      "span_id": "span456",
      "parent_span_id": null,
      "operation_name": "HTTP GET /users",
      "service_name": "api-gateway",
      "start_time": "2025-01-15T10:30:00Z",
      "end_time": "2025-01-15T10:30:00.250Z",
      "status": "OK",
      "attributes": {"http.method": "GET", "http.status_code": "200"},
      "events": []
    }
  ]'
```

Span statuses: `UNSET`, `OK`, `ERROR`.

### Sending Traces via OpenTelemetry (OTLP HTTP+JSON)

rgrab supports trace ingestion in the standard OpenTelemetry format. This allows connecting any application using the OpenTelemetry SDK.

```bash
curl -X POST http://localhost:3000/otlp/v1/traces \
  -H 'Content-Type: application/json' \
  -d '{
    "resourceSpans": [
      {
        "resource": {
          "attributes": [
            {"key": "service.name", "value": {"stringValue": "my-service"}}
          ]
        },
        "scopeSpans": [
          {
            "scope": {"name": "my-lib", "version": "1.0.0"},
            "spans": [
              {
                "traceId": "0af7651916cd43dd8448eb211c80319c",
                "spanId": "b7ad6b7169203331",
                "parentSpanId": "",
                "name": "HTTP GET /api/users",
                "kind": 2,
                "startTimeUnixNano": "1705312200000000000",
                "endTimeUnixNano": "1705312200250000000",
                "attributes": [
                  {"key": "http.method", "value": {"stringValue": "GET"}},
                  {"key": "http.status_code", "value": {"intValue": "200"}}
                ],
                "events": [],
                "status": {"code": 1}
              }
            ]
          }
        ]
      }
    ]
  }'
```

#### Configuring OpenTelemetry SDKs

Point the OTLP HTTP exporter endpoint to rgrab:

**Python** (opentelemetry-sdk):
```python
from opentelemetry.exporter.otlp.proto.http.trace_exporter import OTLPSpanExporter

exporter = OTLPSpanExporter(endpoint="http://localhost:3000/otlp/v1/traces")
```

**Go** (go.opentelemetry.io):
```go
exporter, _ := otlptracehttp.New(ctx,
    otlptracehttp.WithEndpoint("localhost:3000"),
    otlptracehttp.WithURLPath("/otlp/v1/traces"),
    otlptracehttp.WithInsecure(),
)
```

**Node.js** (@opentelemetry/exporter-trace-otlp-http):
```javascript
const exporter = new OTLPTraceExporter({
  url: 'http://localhost:3000/otlp/v1/traces',
});
```

**Rust** (opentelemetry-otlp):
```rust
let exporter = opentelemetry_otlp::SpanExporter::builder()
    .with_http()
    .with_endpoint("http://localhost:3000/otlp/v1/traces")
    .build()?;
```

**Environment variable** (universal):
```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:3000
export OTEL_EXPORTER_OTLP_PROTOCOL=http/json
```

### Sending Logs in Loki Format

```bash
curl -X POST http://localhost:3000/rgrab/api/v1/push \
  -H 'Content-Type: application/json' \
  -d '{
    "streams": [
      {
        "stream": {
          "service": "auth",
          "level": "info",
          "environment": "production"
        },
        "values": [
          ["1705312200000000000", "User logged in"],
          ["1705312201000000000", "Session created", {"trace_id": "abc123"}]
        ]
      }
    ]
  }'
```

Values format: `["timestamp_nanoseconds", "message", {metadata}]`.
The third element (metadata) is optional. May contain `trace_id` and `span_id`.

### Collecting Docker Container Logs

Enable the Docker collector in the server config:

```toml
[docker]
enabled = true
socket = "/var/run/docker.sock"

[[docker.containers]]
name = "my-app"
service = "backend"
environment = "production"
tail = 200

[[docker.containers]]
name = "postgres"
service = "postgres"
environment = "production"
```

The collector automatically attaches to configured containers when they start and detaches when they stop. It reconnects if the Docker daemon restarts.

---

## Querying the API

### Native API

```bash
# Latest 100 logs
curl http://localhost:3000/api/logs

# Latest 10 logs
curl http://localhost:3000/api/logs?limit=10

# All spans for a specific trace
curl http://localhost:3000/api/traces?trace_id=abc123

# Latest 50 spans (all traces)
curl http://localhost:3000/api/traces?limit=50
```

### Loki-compatible API

#### Querying Logs

```bash
# All logs from the auth service
curl 'http://localhost:3000/rgrab/api/v1/query?query={service="auth"}'

# Error-level logs, latest 10
curl 'http://localhost:3000/rgrab/api/v1/query?query={level="error"}&limit=10'

# With direction (oldest first)
curl 'http://localhost:3000/rgrab/api/v1/query?query={service="api"}&direction=forward'
```

Parameters:
| Parameter   | Required | Description                           | Default  |
|-------------|----------|---------------------------------------|----------|
| `query`     | yes      | Label selector: `{key="val"}`         | -        |
| `limit`     | no       | Max number of entries                 | 100      |
| `time`      | no       | Timestamp in nanoseconds (end)        | now      |
| `direction` | no       | `forward` or `backward`               | backward |

#### Time Range Query

```bash
curl 'http://localhost:3000/rgrab/api/v1/query_range?query={service="api"}&start=1705312200000000000&end=1705315800000000000'
```

#### Labels

```bash
# List all label names
curl http://localhost:3000/rgrab/api/v1/labels

# All values for a specific label
curl http://localhost:3000/rgrab/api/v1/label/service/values
```

---

## Label Selector Syntax

| Operator | Meaning            | Example                   |
|----------|--------------------|---------------------------|
| `=`      | Exact match        | `{service="auth"}`        |
| `!=`     | Not equal          | `{level!="debug"}`        |
| `=~`     | Regex match        | `{service=~"api.*"}`      |
| `!~`     | Regex not match    | `{env!~"dev\|staging"}`   |

Examples:
```
{service="auth"}
{service="auth", level="error"}
{service=~"(auth|api)", environment="production"}
{service="auth", level!="trace", level!="debug"}
{}
```

---

## Grafana Integration

1. In Grafana, add a Data Source of type **Loki**
2. URL: `http://<rgrab-host>:3000/rgrab`
3. Save and test the connection
4. In Explore, select the datasource and write queries using label selector format

---

## Examples

### Monitoring Errors

```bash
# All errors in the last hour
curl 'http://localhost:3000/rgrab/api/v1/query_range?query={level="error"}&start='$(date -d '1 hour ago' +%s)000000000'&end='$(date +%s)000000000

# Errors for a specific service
curl 'http://localhost:3000/rgrab/api/v1/query?query={service="api",level=~"error|fatal"}&limit=50'
```

### Tracing a Request

```bash
# All spans for a trace
curl http://localhost:3000/api/traces?trace_id=abc123

# Logs by trace_id
curl http://localhost:3000/api/logs?limit=1000 | jq '.[] | select(.trace_id == "abc123")'
```

### Label Statistics

```bash
# Which services are sending logs
curl http://localhost:3000/rgrab/api/v1/label/service/values

# Which environments exist
curl http://localhost:3000/rgrab/api/v1/label/environment/values
```
