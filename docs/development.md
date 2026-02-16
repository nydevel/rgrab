# Разработка rgrab

## Требования

- **Rust** >= 1.85 (edition 2024)
- **libclang** — необходим для сборки `rocksdb` crate (bindgen)
- **Linux** (основная платформа)

### Установка зависимостей

Ubuntu/Debian:
```bash
sudo apt-get install -y libclang-dev
```

macOS:
```bash
brew install llvm
```

Если `libclang` установлен не в стандартном пути, создайте `.cargo/config.toml`:
```toml
[env]
LIBCLANG_PATH = "/path/to/llvm/lib"
```

## Сборка

```bash
# Проверка компиляции (быстро, без линковки)
cargo check --workspace

# Полная сборка
cargo build --workspace

# Release-сборка сервера
cargo build --release -p server

# Release-сборка TUI
cargo build --release -p tui
```

## Запуск в dev-режиме

### Сервер

```bash
# С дефолтами (порт 3000, БД в ./data/rgrab, log_level=info)
cargo run -p server

# С CLI параметрами
cargo run -p server -- --data-dir ./my-data --listen 127.0.0.1:8080 --log-level debug

# С TOML конфигом
cargo run -p server -- --config my-config.toml

# Справка
cargo run -p server -- --help
```

После запуска доступны все эндпоинты на одном порту:
- Ingest: `POST /v1/logs`, `POST /v1/traces`, `POST /otlp/v1/traces`
- Query: `GET /api/logs`, `GET /api/traces`
- Loki API: `/rgrab/api/v1/*`

### TUI-клиент

```bash
# Подключение к локальному серверу (http://localhost:3000)
cargo run -p tui

# Подключение к удалённому серверу
cargo run -p tui -- --server http://192.168.1.100:3000
```

TUI — это HTTP-клиент, сервер должен быть запущен отдельно.

## Конфигурация сервера

Три способа (приоритет: CLI > TOML > defaults):

### CLI аргументы

| Аргумент       | Описание                          | По умолчанию           |
|----------------|-----------------------------------|------------------------|
| `--config, -c` | Путь к TOML конфигу               | `/etc/rgrab/rgrab.toml`|
| `--data-dir, -d`| Директория RocksDB               | `./data/rgrab`         |
| `--listen, -l` | Адрес и порт                      | `0.0.0.0:3000`         |
| `--log-level`  | Уровень логирования               | `info`                 |

### TOML конфиг

```toml
data_dir = "/var/lib/rgrab"
listen = "0.0.0.0:3000"
log_level = "info"
```

CLI аргументы перекрывают значения из конфига. Если конфиг не найден — используются defaults.

## Проверка кода

```bash
# Линтер
cargo clippy --workspace

# Форматирование
cargo fmt --all

# Проверка форматирования (CI)
cargo fmt --all -- --check
```

## Структура данных в RocksDB

БД создаётся автоматически при первом запуске. Директория содержит файлы RocksDB.

Две column families:
- `logs` — лог-записи (ключ: timestamp + sequence, значение: JSON)
- `spans` — спаны (ключ: `trace_id:span_id`, значение: JSON)

Для очистки БД — удалить директорию:
```bash
rm -rf ./data/rgrab
```

## Быстрая проверка работоспособности

```bash
# 1. Запустить сервер
cargo run -p server &

# 2. Отправить лог
curl -s -X POST http://localhost:3000/v1/logs \
  -H 'Content-Type: application/json' \
  -d '[{
    "timestamp": "2024-01-01T12:00:00Z",
    "level": "INFO",
    "message": "Hello from rgrab",
    "labels": {"service": "test", "env": "dev"},
    "trace_id": null,
    "span_id": null
  }]'

# 3. Прочитать логи через API
curl -s http://localhost:3000/api/logs | python3 -m json.tool

# 4. Открыть TUI
cargo run -p tui
```
