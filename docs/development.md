# Developing rgrab

## Requirements

- **Rust** >= 1.85 (edition 2024)
- **libclang** -- required for building the `rocksdb` crate (bindgen)
- **Linux** (primary platform)

### Installing Dependencies

Ubuntu/Debian:
```bash
sudo apt-get install -y libclang-dev
```

macOS:
```bash
brew install llvm
```

If `libclang` is installed in a non-standard path, create `.cargo/config.toml`:
```toml
[env]
LIBCLANG_PATH = "/path/to/llvm/lib"
```

## Building

```bash
# Check compilation (fast, no linking)
cargo check --workspace

# Full build
cargo build --workspace

# Release build (server)
cargo build --release -p server

# Release build (TUI)
cargo build --release -p tui
```

## Running in Dev Mode

### Server

```bash
# With defaults (port 3000, DB at ./data/rgrab, log_level=info)
cargo run -p server

# With CLI parameters
cargo run -p server -- --data-dir ./my-data --listen 127.0.0.1:8080 --log-level debug

# With TOML config
cargo run -p server -- --config rgrab.toml

# Help
cargo run -p server -- --help
```

Once started, all endpoints are available on a single port:
- Ingest: `POST /v1/logs`, `POST /v1/traces`, `POST /otlp/v1/traces`
- Query: `GET /api/logs`, `GET /api/traces`
- Loki API: `/rgrab/api/v1/*`

### TUI Client

```bash
# Connect to local server (http://localhost:3000)
cargo run -p tui

# Connect to a remote server
cargo run -p tui -- --server http://192.168.1.100:3000
```

The TUI is an HTTP client -- the server must be running separately.

## Server Configuration

Three methods (priority: CLI > TOML > defaults):

### CLI Arguments

| Argument         | Description                       | Default                |
|------------------|-----------------------------------|------------------------|
| `--config, -c`   | Path to TOML config              | `/etc/rgrab/rgrab.toml`|
| `--data-dir, -d` | RocksDB directory                | `./data/rgrab`         |
| `--listen, -l`   | Listen address and port          | `0.0.0.0:3000`         |
| `--log-level`    | Log level                        | `info`                 |

### TOML Config

```toml
data_dir = "/var/lib/rgrab"
listen = "0.0.0.0:3000"
log_level = "info"

[docker]
enabled = true
socket = "/var/run/docker.sock"

[[docker.containers]]
name = "my-app"
service = "my-app"
environment = "production"
tail = 200
```

CLI arguments override values from the config file. If the config file is not found, defaults are used.

## Code Quality

```bash
# Linter
cargo clippy --workspace

# Formatting
cargo fmt --all

# Check formatting (CI)
cargo fmt --all -- --check
```

## RocksDB Data Structure

The database is created automatically on first startup. The directory contains RocksDB files.

Two column families:
- `logs` -- log entries (key: timestamp + sequence, value: JSON)
- `spans` -- spans (key: `trace_id:span_id`, value: JSON)

To reset the database, delete the directory:
```bash
rm -rf ./data/rgrab
```

## Quick Smoke Test

```bash
# 1. Start the server
cargo run -p server -- --config rgrab.toml &

# 2. Send a log entry
curl -s -X POST http://localhost:3000/v1/logs \
  -H 'Content-Type: application/json' \
  -d '[{
    "timestamp": "2025-01-01T12:00:00Z",
    "level": "INFO",
    "message": "Hello from rgrab",
    "labels": {"service": "test", "env": "dev"},
    "trace_id": null,
    "span_id": null
  }]'

# 3. Read logs via API
curl -s http://localhost:3000/api/logs | python3 -m json.tool

# 4. Open TUI
cargo run -p tui
```
