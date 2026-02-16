# rgrab Architecture

## Overview

rgrab is a lightweight observability backend written in Rust. It accepts, stores, and serves logs and distributed traces. It provides a Loki-compatible API for Grafana integration, supports OpenTelemetry (OTLP) trace ingestion, and collects logs from Docker containers. Includes a TUI client for browsing logs and traces in the terminal.

## Project Structure

```
rgrab/
├── Cargo.toml                # workspace root
├── rgrab.toml                # server config
├── rgrab.example.toml        # example config with comments
├── docs/                     # documentation
├── packaging/                # systemd, config, scripts
│   ├── config/rgrab.toml     # default config for deb package
│   ├── systemd/              # systemd unit
│   └── scripts/              # postinst, prerm
└── crates/
    ├── common/               # data models, types, parsers
    ├── storage/              # storage layer (RocksDB)
    ├── collector/            # data ingestion API
    ├── docker-collector/     # Docker container log collection
    ├── web/                  # query API + Loki API
    ├── server/               # unified server binary
    └── tui/                  # TUI client (rgrab-tui)
```

## Crates

### common

Shared data types used by all other crates.

| Module             | Purpose                                                   |
|--------------------|-----------------------------------------------------------|
| `log.rs`           | `LogEntry`, `LogLevel` -- log entry structure              |
| `span.rs`          | `Span`, `SpanStatus`, `SpanEvent` -- distributed tracing   |
| `loki.rs`          | Loki-compatible request/response types, `LabelMatcher`     |
| `label_selector.rs`| Label selector parser `{key="val", k2!="v2"}`             |
| `otlp.rs`          | OTLP types and conversion to internal Span model           |

### storage

Persistent storage based on RocksDB.

**RocksStore** -- main implementation. Uses two column families:

- **logs** -- log entries. Key: `[8B timestamp_nanos BE][8B sequence BE]` (16 bytes). Natural chronological ordering.
- **spans** -- spans. Key: `"{trace_id}:{span_id}"`. Enables prefix iteration by trace_id.

Values are stored as JSON (`serde_json`).

All public methods are `async`, internally using `tokio::task::spawn_blocking` to avoid blocking the async runtime.

### collector

Data ingestion service. Endpoints:

- `POST /v1/logs` -- accepts array of `LogEntry` (JSON)
- `POST /v1/traces` -- accepts array of `Span` (JSON)
- `POST /otlp/v1/traces` -- accepts traces in OpenTelemetry format (OTLP HTTP+JSON)

### docker-collector

Docker container log collection library. Connects to the Docker daemon via unix socket, discovers configured containers, streams their stdout/stderr in follow mode, parses log levels, and writes directly to RocksStore.

- Watches Docker events (start/die) -- automatically attaches to containers when they start
- Reconnects on Docker daemon restarts
- Configurable per-container: service name, environment, tail lines

### web

Query service. Two API sets:

**Native API** (`api.rs`):
- `GET /api/logs?limit=N`
- `GET /api/traces?trace_id=ID&limit=N`

**Loki-compatible API** (`loki_api.rs`):
- `POST /rgrab/api/v1/push`
- `GET /rgrab/api/v1/query`
- `GET /rgrab/api/v1/query_range`
- `GET /rgrab/api/v1/labels`
- `GET /rgrab/api/v1/label/{name}/values`

### server

Unified server binary (`rgrab`), combining collector + web + loki_api + docker-collector in a single process. Configured via CLI arguments (clap) and/or TOML config.

Required because RocksDB locks the database directory -- two processes cannot open the same database simultaneously.

### tui

TUI client (`rgrab-tui`) -- a standalone binary built with ratatui + crossterm. Connects to the server via HTTP, provides an interface for browsing logs and traces in the terminal.

- Two tabs: Logs and Traces
- Sidebar with label filtering (service, environment)
- Text search
- Live tail with auto-refresh
- Trace expansion with waterfall span visualization
- Connection error display when server is unavailable

## Data Flow

```
                    ┌──────────────────┐
                    │   Applications   │
                    │ (SDK, promtail)  │
                    └────────┬─────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
  POST /v1/logs     POST /v1/traces    POST /otlp/v1/traces
  POST /rgrab/api/v1/push              (OpenTelemetry)
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │
                    ┌────────▼─────────┐    ┌──────────────────┐
                    │   RocksStore     │◄───│ Docker Collector  │
                    │  (column families│    │ (container logs)  │
                    │   logs / spans)  │    └──────────────────┘
                    └────────┬─────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
  GET /api/logs     GET /api/traces    GET /rgrab/api/v1/query*
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
         rgrab-tui      Grafana          curl
         (TUI)       (Loki source)
```

## Configuration

The server supports three configuration methods (priority: CLI > TOML > defaults):

1. **CLI arguments** -- `rgrab --data-dir /path --listen 0.0.0.0:3000 --log-level debug`
2. **TOML config** -- `rgrab --config /etc/rgrab/rgrab.toml`
3. **Defaults** -- `data_dir=./data/rgrab`, `listen=0.0.0.0:3000`, `log_level=info`

## Key Decisions

1. **RocksDB** -- embedded database, no separate server required. Persistence out of the box.
2. **Single binary** -- simplifies deployment, avoids database lock contention.
3. **Loki-compatible API** -- custom paths (`/rgrab/api/v1/...`), but response format is identical to Loki. Enables Grafana as a UI.
4. **OTLP support** -- trace ingestion in standard OpenTelemetry format (HTTP+JSON).
5. **Docker collector** -- built-in container log collection, no external agent needed.
6. **TUI instead of Web UI** -- terminal client, no browser required, works over SSH.
7. **Label-based filtering** -- logs are filtered by labels, like Loki. Supports `=`, `!=`, `=~`, `!~`.
8. **clap + TOML** -- configuration via CLI and/or file, CLI takes precedence over config.

## Technology Stack

| Component       | Technology                |
|-----------------|---------------------------|
| Language        | Rust (edition 2024)       |
| Async runtime   | Tokio                     |
| HTTP framework  | Axum 0.8                  |
| Storage         | RocksDB 0.22              |
| Serialization   | serde / serde_json        |
| CLI             | clap 4                    |
| Config          | toml 0.8                  |
| Docker API      | bollard 0.18              |
| TUI             | ratatui 0.29 + crossterm 0.28 |
| HTTP client     | reqwest 0.12              |
| Logging         | tracing / tracing-subscriber |
| Error handling  | anyhow                    |
