# rgrab

Lightweight observability backend for logs and distributed traces.

Accepts logs via HTTP push, collects from Docker containers, receives OpenTelemetry (OTLP) traces, and provides a Loki-compatible query API. Uses embedded RocksDB for storage. Comes with a built-in terminal UI.

![photo_2026-02-16_23-33-01](https://github.com/user-attachments/assets/47c9a943-396b-4f49-9487-ae3ae4628d79)


## Features

- **Log ingestion** -- HTTP push API, Loki-compatible push, Docker container log collection
- **Distributed tracing** -- OTLP trace ingestion with span correlation
- **Loki-compatible API** -- query logs with `{service="my-app", level="error"}` selectors
- **Docker collector** -- streams stdout/stderr from configured containers, auto-detects log levels
- **Terminal UI** -- browse logs and traces, filter by service/environment, search, live tail
- **Embedded storage** -- RocksDB, no external dependencies
- **Single binary** -- server, collector, and Docker watcher in one process

## Quick Start

```bash
# Build
cargo build --release

# Run server with local config
cargo run -p server -- --config rgrab.toml

# Open TUI
cargo run -p tui
```

## Configuration

Copy `rgrab.example.toml` to `rgrab.toml` and adjust:

```toml
data_dir = "./data/rgrab"
listen = "0.0.0.0:3000"
log_level = "info"

[docker]
enabled = true
socket = "/var/run/docker.sock"

[[docker.containers]]
name = "my-app"
service = "backend"
environment = "production"

[[docker.containers]]
name = "nginx"
service = "nginx"
environment = "production"
```

### Docker Collector

When `docker.enabled = true`, the server connects to the Docker daemon and streams logs from the listed containers. Each log entry gets labels:

| Label | Source |
|-------|--------|
| `service` | `service` field from config, or container name |
| `environment` | `environment` field from config |
| `container_name` | Docker container name |
| `container_id` | Short container ID |
| `image` | Docker image name |
| `stream` | `stdout` or `stderr` |

The collector automatically attaches to new containers when they start and detaches when they stop.

## API

### Log Ingestion

```bash
# Push logs (JSON array of LogEntry)
curl -X POST http://localhost:3000/v1/logs \
  -H 'Content-Type: application/json' \
  -d '[{"timestamp":"2025-01-01T00:00:00Z","level":"INFO","message":"hello","labels":{"service":"test"}}]'

# Loki-compatible push
curl -X POST http://localhost:3000/rgrab/api/v1/push \
  -H 'Content-Type: application/json' \
  -d '{"streams":[{"stream":{"service":"test"},"values":[["1704067200000000000","hello"]]}]}'
```

### OTLP Traces

```bash
# Push traces (OpenTelemetry JSON format)
curl -X POST http://localhost:3000/otlp/v1/traces \
  -H 'Content-Type: application/json' \
  -d @traces.json
```

### Query

```bash
# List labels
curl http://localhost:3000/rgrab/api/v1/labels

# Label values
curl http://localhost:3000/rgrab/api/v1/label/service/values

# Query logs with selector
curl 'http://localhost:3000/rgrab/api/v1/query?query={service="my-app"}&limit=100'

# Query range
curl 'http://localhost:3000/rgrab/api/v1/query_range?query={level="error"}&start=1704067200&end=1704153600'
```

## TUI

```bash
cargo run -p tui -- --server http://localhost:3000
```

| Key | Action |
|-----|--------|
| `Tab` | Switch between Logs / Traces |
| `j/k` | Scroll down / up |
| `h/l` | Focus sidebar / main panel |
| `Enter` | Select filter / expand trace |
| `/` | Search |
| `1-6` | Toggle log levels (TRACE..FATAL) |
| `L` | Toggle live tail |
| `r` | Refresh |
| `s` | Toggle sort order |
| `q` | Quit |

## Project Structure

```
crates/
  common/            Shared types (LogEntry, Span, label selectors)
  collector/         HTTP ingestion endpoints
  docker-collector/  Docker container log collection
  storage/           RocksDB persistence layer
  web/               Loki-compatible query API
  server/            Main binary (merges all components)
  tui/               Terminal UI client
```

## Installation

### From source

```bash
cargo install --path crates/server
cargo install --path crates/tui
```

### Debian package

```bash
# Install cargo-deb (one-time)
cargo install cargo-deb

# Build release binaries and package
cargo build --release
cargo deb -p server --no-build

# Install locally
sudo dpkg -i target/debian/rgrab_*.deb
sudo systemctl enable --now rgrab
```

The deb package installs:
- `/usr/bin/rgrab` -- server binary
- `/usr/bin/rgrab-tui` -- terminal UI
- `/etc/rgrab/rgrab.toml` -- configuration (preserved on upgrade)
- systemd service `rgrab`

### Deploy to server

The `deploy.sh` script builds the deb package and deploys it to a remote server over SSH:

```bash
# Build, upload, install, and restart in one command
infra/deploy.sh root@your-server.com

# Or with a custom SSH user
infra/deploy.sh deploy@192.168.1.10
```

The script does the following:
1. Builds release binaries (`cargo build --release`)
2. Creates a deb package (`cargo deb`)
3. Copies the package to the server via `scp`
4. Installs it with `dpkg -i` and restarts the service

Requirements:
- SSH access to the server (key-based auth recommended)
- `cargo-deb` installed locally (`cargo install cargo-deb`)
- The server must be Debian/Ubuntu-based

## License

MIT
